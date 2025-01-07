mod pipe;
mod registers;

pub use { pipe::*, registers::* };

use std::ffi::CString;
use std::process::exit;

use nix::sys::{
    signal,
    signal::Signal,
    ptrace,
    wait,
};
use nix::unistd::Pid;
use libc::{
    fork,
    execvp
};

pub type Result<T> = std::result::Result<T, GadbErr>;

#[derive(Debug, PartialEq)]
pub struct GadbErr {
    msg: String
}

pub fn error<T>(msg: &str) -> Result<T> {
    Err(GadbErr {
        msg: String::from(msg)
    })
}

pub fn error_os<T>(msg: &str) -> Result<T> {
    Err(GadbErr {
        msg: os_error_with_prefix(msg)
    })
}

pub fn os_error_with_prefix(prefix: &str) -> String {
    String::from(prefix) + &": " + &std::io::Error::last_os_error().to_string()
}
impl std::fmt::Display for GadbErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
#[derive(PartialEq, Clone, Debug)]
pub enum ProcessState {
    Stopped,
    Running,
    Exited,
    Terminated    
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match *self {
            ProcessState::Stopped => "stopped",
            ProcessState::Running => "running",
            ProcessState::Exited => "exited",
            ProcessState::Terminated => "terminated",
        })
    }
}

#[derive(Debug)]
pub enum StopInfo {
    Signal(Signal),
    ExitCode(i32)
}

impl std::fmt::Display for StopInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopInfo::Signal(s) => write!(f, "signal {}", s),
            StopInfo::ExitCode(c) => write!(f, "exit code {}", c)
        }
    }
}

#[derive(Debug)]
pub struct StopReason {
    newstate: ProcessState,
    info: StopInfo
}

impl StopReason {
    /*
    pub enum WaitStatus {
        Exited(Pid, i32),
        Signaled(Pid, Signal, bool),
        Stopped(Pid, Signal),
        PtraceEvent(Pid, Signal, c_int),
        PtraceSyscall(Pid),
        Continued(Pid),
        StillAlive,
    }
    */
    fn from_wait_status(status: wait::WaitStatus) -> Self {
        match status {
            wait::WaitStatus::Exited(_, code) => {
                Self {
                    newstate: ProcessState::Exited,
                    info: StopInfo::ExitCode(code)
                }
            },
            wait::WaitStatus::Signaled(_, signal, _) => {
                Self {
                    newstate: ProcessState::Terminated,
                    info: StopInfo::Signal(signal)
                }
            },
            wait::WaitStatus::Stopped(_, signal) => {
                Self {
                    newstate: ProcessState::Stopped,
                    info: StopInfo::Signal(signal)
                }
            },
            _ => { panic!("unknown status: {:?}", status) }
        }
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.newstate {
            ProcessState::Stopped => write!(f, "stopped with {}", self.info),
            ProcessState::Running => write!(f, "running"),
            ProcessState::Exited => write!(f, "exited with {}", self.info),
            ProcessState::Terminated => write!(f, "terminated with {}", self.info),
        }
    }
}

#[derive(Debug)]
pub struct Process {
    pub pid: Pid,
    pub autoterm: bool,
    pub attached: bool,
    pub state: ProcessState
}

impl Process {
    pub fn launch(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new(), true);
    }
    pub fn launch_noattach(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new(), false);
    }
    pub fn launch_args(cmd: &str, args: Vec::<String>, attach: bool) -> Result<Self> {
        let Ok(cmd_c) = CString::new(cmd) else {
            return error("could not read cmd");
        };
        let mut pipe = Pipe::pipe(true).unwrap();
        let pid: i32;
        unsafe {
            pid = fork();
        }

        fn exit_with_error(pipe: &mut Pipe, prefix: &str) {
            let msg = os_error_with_prefix(prefix);
            let _ = pipe.write(msg.as_bytes().to_vec());
            exit(-1);
        }

        if pid == 0 {
            pipe.close_read();
            if attach && ptrace::traceme().is_err() {
                exit_with_error(&mut pipe, &"error calling PTRACE_TRACEME");
            }

            let mut args_cstr: Vec<CString> = args.iter().map(|s| CString::new(s.clone()).unwrap()).collect();
            args_cstr.insert(0, CString::new(cmd).unwrap());
            let mut args_ptr: Vec<*const i8> = args_cstr.iter().map(|s| s.as_c_str().as_ptr()).collect();
            args_ptr.push(std::ptr::null() as *const i8);
            
            unsafe {
                let _ = execvp(cmd_c.as_c_str().as_ptr(), args_ptr.as_ptr());
            }
            exit_with_error(&mut pipe, "error calling exec");
        }

        pipe.close_write();
        let data = pipe.read();
        if let Ok(data) = data {
            if data.len() > 0 {
                let _ = wait::waitpid(Some(Pid::from_raw(pid)), None);
                return error(&String::from_utf8_lossy(data.as_slice()));
            }
        }
        pipe.close_read();
        let mut p = Self {
            pid: Pid::from_raw(pid),
            autoterm: true,
            attached: attach,
            state: ProcessState::Running
        };
        if attach {
            let _ = p.wait_on_signal();
        }
        Ok(p)
    }

    pub fn attach(pid: i32) -> Result<Self> {
        if pid <= 0 {
            return error("invalid pid: 0");
        }
        let pid = Pid::from_raw(pid);
        let res = ptrace::attach(pid);
        if res.is_err() {
            return error(res.err().unwrap().desc());
        }
        let mut p = Self {
            pid,
            autoterm: false,
            attached: true,
            state: ProcessState::Running
        };
        let _ = p.wait_on_signal();
        Ok(p)
    }

    pub fn wait_on_signal(&mut self) -> Result<StopReason> {
        // passing None is equivalent to using 0 for the options
        let res = wait::waitpid(self.pid, None);
        let Ok(status) = res else {
            return error_os("could not wait on signal");
        };
        let reason = StopReason::from_wait_status(status);
        self.state = reason.newstate.clone();
        Ok(reason)
    }

    pub fn res(&mut self) {
        let _ = self.resume();
    }

    pub fn resume(&mut self) -> Result<()> {
        let res = ptrace::cont(self.pid, None);
        // TODO: error handling
        if res.is_err() {
            return error("could not resume");
        }
        self.state = ProcessState::Running;
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.pid == Pid::from_raw(0) {
            return;
        }
        if self.attached {
            if self.state == ProcessState::Running {
                let _ = signal::kill(self.pid, Signal::SIGKILL);
                let _ = wait::waitpid(self.pid, None);
            }
            let _ = ptrace::detach(self.pid, None);
            let _ = signal::kill(self.pid, Signal::SIGCONT);
        }
        if self.autoterm {
            let _ = signal::kill(self.pid, Signal::SIGKILL);
            let _ = wait::waitpid(self.pid, None);
        }
    }
}
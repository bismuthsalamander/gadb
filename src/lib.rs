#[cfg(test)]
mod tests;

use std::ffi::CString;

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

#[derive(Debug)]
pub struct GadbErr {
    msg: String
}

pub fn error<T>(msg: &str) -> Result<T> {
    Err(GadbErr {
        msg: String::from(msg)
    })
}

impl std::fmt::Display for GadbErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
#[derive(PartialEq, Clone)]
pub enum ProcessState {
    Stopped,
    Running,
    Exited,
    Terminated    
}

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

pub struct Process {
    pub pid: Pid,
    autoterm: bool,
    pub state: ProcessState
}

impl Process {
    pub fn launch(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new());
    }
    pub fn launch_args(cmd: &str, args: Vec::<String>) -> Result<Self> {
        let Ok(cmd_c) = CString::new(cmd) else {
            return error("could not read cmd");
        };
        let pid: i32;
        unsafe {
            pid = fork();
        }
        if pid == 0 {
            let _ = ptrace::traceme();
            let args_cstr: Vec<CString> = args.iter().map(|s| CString::new(s.clone()).unwrap()).collect();
            let mut args_ptr: Vec<*const i8> = args_cstr.iter().map(|s| s.as_c_str().as_ptr()).collect();
            args_ptr.push(std::ptr::null() as *const i8);
            unsafe {
                // let r = execlp(cmd_c.as_c_str().as_ptr(), cmd_c.as_c_str().as_ptr(), std::ptr::null() as *const i8);
                let _r = execvp(cmd_c.as_c_str().as_ptr(), args_ptr.as_ptr());
                return error(&format!("error calling exec: {}", std::io::Error::last_os_error().to_string()));
            }
        }

        let mut p = Self {
            pid: Pid::from_raw(pid),
            autoterm: true,
            state: ProcessState::Running
        };
        let _ = p.wait_on_signal();
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
            state: ProcessState::Running
        };
        let _ = p.wait_on_signal();
        Ok(p)
    }
    pub fn wait_on_signal(&mut self) -> StopReason {
        // passing None is equivalent to using 0 for the options
        let res = wait::waitpid(self.pid, None);
        let Ok(status) = res else {
            eprintln!("could not wait on signal: {}", res.err().unwrap().desc());
            std::process::exit(-1);
        };
        let reason = StopReason::from_wait_status(status);
        self.state = reason.newstate.clone();
        reason
    }
    pub fn resume(&mut self) {
        let res = ptrace::cont(self.pid, None);
        // TODO: error handling
        if res.is_err() {
            eprintln!("could not resume");
            std::process::exit(-1);
        }
        self.state = ProcessState::Running;
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.pid == Pid::from_raw(0) {
            return;
        }
        if self.state == ProcessState::Running {
            let _ = signal::kill(self.pid, Signal::SIGKILL);
            let _ = wait::waitpid(self.pid, None);
        }
        let _ = ptrace::detach(self.pid, None);
        let _ = signal::kill(self.pid, Signal::SIGCONT);
        if self.autoterm {
            let _ = signal::kill(self.pid, Signal::SIGKILL);
            let _ = wait::waitpid(self.pid, None);
        }
    }
}
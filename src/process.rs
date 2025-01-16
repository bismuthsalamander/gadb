use crate::{
    Result,
    error,
    error_os,
    os_error_with_prefix,
    registers::*,
    register_info::*,
    pipe::Pipe
};

use nix::{
    sys::wait,
    sys::signal,
    sys::signal::Signal,
    sys::ptrace,
    unistd::Pid,
};

use libc::{
    fork,
    execvp,
    user_regs_struct,
    user_fpregs_struct
};

use std::ffi::CString;
use std::process::exit;

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
    pub state: ProcessState,
    registers: Registers
}

impl Process {
    pub fn launch(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new(), true, None);
    }

    pub fn launch_noattach(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new(), false, None);
    }

    pub fn launch_args(cmd: &str, args: Vec::<String>, attach: bool, stdout: Option<std::os::fd::RawFd>) -> Result<Self> {
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
            if let Some(raw_fd) = stdout {
                unsafe {
                    if libc::dup2(raw_fd, libc::STDOUT_FILENO) < 0 {
                        exit_with_error(&mut pipe, &"error calling dup2");
                    }
                }
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
            state: ProcessState::Running,
            registers: Registers::empty()
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
            state: ProcessState::Running,
            registers: Registers::empty()
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

        if self.attached && self.state == ProcessState::Stopped {
            let _ = self.read_all_registers();
        }
        Ok(reason)
    }

    pub fn res(&mut self) {
        let _ = self.resume();
    }

    pub fn resume(&mut self) -> Result<()> {
        let res = ptrace::cont(self.pid, None);
        if res.is_err() {
            return error("could not resume");
        }
        self.state = ProcessState::Running;
        Ok(())
    }

    pub fn get_fpregs(&self) -> Result<user_fpregs_struct> {
        //ptrace_get_data::<user_regs_struct>(Request::PTRACE_GETREGS, pid)
        let mut data = std::mem::MaybeUninit::<user_fpregs_struct>::uninit();
        let res = unsafe {
            libc::ptrace(
                ptrace::Request::PTRACE_GETFPREGS as libc::c_uint,
                libc::pid_t::from(self.pid),
                std::ptr::null_mut::<user_fpregs_struct>(),
                data.as_mut_ptr(),
            )
        };

        if nix::errno::Errno::result(res).is_err() {
            return error("error in get_fpregs");
        }
        Ok(unsafe { data.assume_init() })
    }

    fn read_all_registers(&mut self) -> Result<()> {
        if let Ok(regs) = ptrace::getregs(self.pid) {
            self.registers.userdata.regs = regs;
        }
        if let Ok(fpregs) = self.get_fpregs() {
            self.registers.userdata.i387 = fpregs;
        }
        
        for (i, id) in DR_IDS.iter().enumerate() {
            let ri = register_by_id(id).unwrap();
            if let Ok(val) = ptrace::read_user(self.pid, ri.offset as *mut libc::c_void) {
                self.registers.userdata.u_debugreg[i] = val as u64;
            } else {
                return error_os("could not read debug register");
            }
        }
        Ok(())
    }

    pub fn write_reg(&mut self, ri: &RegInfo, val: ValUnion) {
        self.registers.write(ri, val);
        if ri.rtype == RegisterType::Fpr {
            let _ = self.write_fprs(self.registers.userdata.i387.clone());
            return;
        }
        let offset = ri.offset & !0b111;
        let bytes = self.registers.get_clong_at(offset);
        let _ = ptrace::write_user(self.pid, offset as *mut libc::c_void, bytes);
    }

    pub fn write_fprs(&mut self, fpregs: user_fpregs_struct) -> Result<()> {
        let res = unsafe {
            let res = libc::ptrace(
                ptrace::Request::PTRACE_SETFPREGS as libc::c_uint,
                libc::pid_t::from(self.pid),
                std::ptr::null_mut::<libc::c_void>(),
                &fpregs as *const user_fpregs_struct as *const libc::c_void,
            );
            dbg!(&res);
            res
        };
        if nix::errno::Errno::result(res).is_err() {
            return error("error in write_fprs");
        }
        Ok(())
    }

    pub fn write_gprs(&mut self, regs: &user_regs_struct) -> Result<()> {
        if ptrace::setregs(self.pid, regs.clone()).is_err() {
            return error("error calling PTRACE_SETREGS");
        }
        Ok(())
    }

    pub fn regs(&self) -> &Registers {
        &self.registers
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
use crate::breakpoints::{BreakSite, VirtAddr, StopPoint};
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
    sys::personality,
    sys::signal,
    sys::ptrace,
    sys::uio,
    unistd::Pid,
};

use libc::{
    fork,
    execvp,
    user_regs_struct,
    user_fpregs_struct
};

use std::cmp::min;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::IoSliceMut;
use std::process::exit;

const INT3: u8 = 0xcc;

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

#[derive(PartialEq, Debug)]
pub enum StopInfo {
    Signal(signal::Signal),
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
    registers: Registers,
    breaksites: HashMap::<usize, BreakSite>,
    next_breaksite_id: usize
}

trait BreakSiteId {
    fn find_breaksite<'a>(&self, p: &'a Process) -> Option<&'a BreakSite>;
    fn find_breaksite_mut<'a>(&self, p: &'a mut Process) -> Option<&'a mut BreakSite>;
}

impl BreakSiteId for &BreakSite {
    fn find_breaksite<'a>(&self, p: &'a Process) -> Option<&'a BreakSite> {
        p.breaksite_by_id(self.id)
    }
    fn find_breaksite_mut<'a>(&self, p: &'a mut Process) -> Option<&'a mut BreakSite> {
        p.breaksite_by_id_mut(self.id)
    }
}

impl BreakSiteId for &mut BreakSite {
    fn find_breaksite<'a>(&self, p: &'a Process) -> Option<&'a BreakSite> {
        p.breaksite_by_id(self.id)
    }
    fn find_breaksite_mut<'a>(&self, p: &'a mut Process) -> Option<&'a mut BreakSite> {
        p.breaksite_by_id_mut(self.id)
    }
}

impl BreakSiteId for usize {
    fn find_breaksite<'a>(&self, p: &'a Process) -> Option<&'a BreakSite> {
        p.breaksite_by_id(*self)
    }
    fn find_breaksite_mut<'a>(&self, p: &'a mut Process) -> Option<&'a mut BreakSite> {
        p.breaksite_by_id_mut(*self)
    }
}
impl BreakSiteId for VirtAddr {
    fn find_breaksite<'a>(&self, p: &'a Process) -> Option<&'a BreakSite> {
        p.breaksite_at_va(*self)
    }
    fn find_breaksite_mut<'a>(&self, p: &'a mut Process) -> Option<&'a mut BreakSite> {
        p.breaksite_at_va_mut(*self)
    }
}

impl Process {
    pub fn launch(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new(), true, None);
    }

    pub fn launch_noattach(cmd: &str) -> Result<Self> {
        return Self::launch_args(cmd, Vec::new(), false, None);
    }

    pub fn breaksites(&self) -> Vec::<&BreakSite> {
        self.breaksites.values().collect()
    }

    pub fn breaksite_by_id(&self, id: usize) -> Option<&BreakSite> {
        self.breaksites.get(&id)
    }

    pub fn breaksite_at_va(&self, va: VirtAddr) -> Option<&BreakSite> {
        for bs in self.breaksites.values() {
            if bs.addr() == va {
                return Some(bs);
            }
        }
        None
    }

    pub fn breaksite_by_id_mut(&mut self, id: usize) -> Option<&mut BreakSite> {
        self.breaksites.get_mut(&id)
    }

    pub fn breaksite_at_va_mut(&mut self, va: VirtAddr) -> Option<&mut BreakSite> {
        for bs in self.breaksites.values_mut() {
            if bs.addr() == va {
                return Some(bs);
            }
        }
        None
    }
    
    pub fn set_pc(&mut self, va: VirtAddr) {
        self.write_reg(&RValue::from_id(va.0, RegisterId::rip));
    }

    pub fn get_pc(&self) -> VirtAddr {
        self.registers.userdata.regs.rip.into()
    }

    pub fn enable_breaksite_by<T: BreakSiteId>(&mut self, id: T) -> Result<()> {
        let pid = self.pid.clone();
        let Some(bs) = id.find_breaksite_mut(self) else {
            return error("could not find breaksite");
        };
        Self::enable_breaksite(pid, bs)
    }

    pub fn enable_breaksite(pid: Pid, bs: &mut BreakSite) -> Result<()> {
        let Ok(data) = ptrace::read(pid, bs.addr().into()) else {
            return error("could not PTRACE_PEEKDATA");
        };
        bs.saved_data = Some(data.to_le_bytes()[0..1].into());
        let data = (data & !0xff) | INT3 as i64;
        if ptrace::write(pid, bs.addr().into(), data).is_err() {
            return error("could not PTRACE_POKEDATA");
        }
        bs.set_enabled();
        Ok(())
    }

    pub fn disable_breaksite_by<T: BreakSiteId>(&mut self, id: T) -> Result<()> {
        let pid = self.pid.clone();
        let Some(bs) = id.find_breaksite_mut(self) else {
            return error("could not find breaksite");
        };
        Self::disable_breaksite(pid, bs)
    }

    pub fn disable_breaksite(pid: Pid, bs: &mut BreakSite) -> Result<()> {
        let Ok(mut data) = ptrace::read(pid, bs.addr().into()) else {
            return error("could not PTRACE_PEEKDATA");
        };
        if let Some(saved) = &bs.saved_data {
            if saved.len() > 8 {
                return error(&format!("I have {} bytes of saved data; 8 should be the max", saved.len()));
            }
            for (i, byte) in saved.iter().enumerate() {
                data = (data & !(0xff << (i*8))) | ((byte << (i*8)) as i64);
            }
        }
        if ptrace::write(pid, bs.addr().into(), data).is_err() {
            return error("could not PTRACE_POKEDATA");
        }
        bs.set_disabled();
        Ok(())
    }

    pub fn create_breaksite(&mut self, va: VirtAddr) -> Result<usize> {
        if let Some(existing) = self.breaksite_at_va(va) {
            return error(&format!("breakpoint already exists at that address (id {})", existing.va));
        }
        let id = self.next_breaksite_id;
        self.next_breaksite_id += 1;
        self.breaksites.insert(id, BreakSite::new(id, va));
        Ok(id)
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

            let Ok(pers) = personality::get() else {
                exit_with_error(&mut pipe, "could not get process personality");
                return error("could not get process personality");
            };
            let Ok(_) = personality::set(pers | personality::Persona::ADDR_NO_RANDOMIZE) else {
                exit_with_error(&mut pipe, "could not set process personality to disable ASLR");
                return error("could not set process personality");
            };

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
            registers: Registers::empty(),
            breaksites: HashMap::new(),
            next_breaksite_id: 0
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
            registers: Registers::empty(),
            breaksites: HashMap::new(),
            next_breaksite_id: 0
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

        let instr_begin = self.get_pc() - 1u64;
        if reason.info == StopInfo::Signal(signal::Signal::SIGTRAP) && self.breaksite_at_va(instr_begin).is_some() {
            self.set_pc(instr_begin);
        }
        Ok(reason)
    }

    pub fn res(&mut self) {
        let _ = self.resume();
    }

    pub fn resume(&mut self) -> Result<()> {
        let pc = self.get_pc();
        if let Some(_) = self.breaksite_at_va_mut(pc) {
            self.disable_breaksite_by(pc)?;
            if ptrace::step(self.pid, None).is_err() {
                return error("could not PTRACE_SINGLESTEP");
            }
            if wait::waitpid(self.pid, None).is_err() {
                return error_os("could not waitpid");
            }
            self.enable_breaksite_by(pc)?;
        }
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
            let ri = register_by_id(*id).unwrap();
            if let Ok(val) = ptrace::read_user(self.pid, ri.offset as *mut libc::c_void) {
                self.registers.userdata.u_debugreg[i] = val as u64;
            } else {
                return error_os("could not read debug register");
            }
        }
        Ok(())
    }

    pub fn write_reg(&mut self, rv: &RValue) {
        self.registers.write(rv);
        if rv.ri.rtype == RegisterType::Fpr {
            let _ = self.write_fprs(self.registers.userdata.i387.clone());
            return;
        }
        let offset = rv.ri.offset & !0b111;
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
    
    pub fn enable_all_breaksites(&mut self) -> usize {
        let mut out = 0;
        for (_, bs) in self.breaksites.iter_mut() {
            if bs.enabled() && Self::disable_breaksite(self.pid, bs).is_ok() {
                out += 1;
            }
        }
        out
    }

    pub fn disable_all_breaksites(&mut self) -> usize {
        let mut out = 0;
        for (_, bs) in self.breaksites.iter_mut() {
            if !bs.enabled() && Self::enable_breaksite(self.pid, bs).is_ok() {
                out += 1;
            }
        }
        out
    }
    
    pub fn clear_all_breaksites(&mut self) -> usize {
        let sz = self.breaksites.values().filter(|bs| bs.enabled()).count();
        self.disable_all_breaksites();
        self.breaksites.clear();
        sz
    }

    pub fn clear_breaksite(&mut self, id: usize) -> Result<()> {
        let Some(mut bs) = self.breaksites.remove(&id) else {
            return error("could not find breaksite by id");
        };
        Self::disable_breaksite(self.pid, &mut bs)
    }
/*
    fn split_into_slices<T>(slice: &mut [T], slice_size: usize) -> Vec<&mut [T]> {
        let (first, rest) = slice.split_at_mut(slice_size);
        let mut result = vec![first];
        result.extend(Self::split_into_slices(rest, n - 1));
        result
    }

    
    pub fn read_memory(
        &self,
        start: VirtAddr,
        count: usize
    ) -> Result<Vec<u8>> {
        // Create a buffer to hold all the data
        let mut buffer = vec![0u8; count];
        
        // Create local iovecs using our safe split function
        let local_slices = Self::split_into_slices(&mut buffer, 8);
        let local_iovecs: Vec<IoSliceMut> = local_slices
            .into_iter()
            .map(|slice| IoSliceMut::new(slice))
            .collect();
    
        // Create remote iovecs
        let remote_iovecs: Vec<uio::RemoteIoVec> = remote_addresses
            .iter()
            .map(|&addr| uio::RemoteIoVec {
                base: addr,
                len: chunk_size,
            })
            .collect();
    
        // Perform the read
        let bytes_read = uio::process_vm_readv(
            self.pid,
            &local_iovecs,
            &remote_iovecs,
        )?;
    
        if bytes_read != total_size {
            return error("incomplete read");
        }
    
        Ok(buffer)
    }
*/

    pub fn read_memory(&self, start: VirtAddr, count: usize) -> Result<Vec::<u8>> {
        unsafe {
            let page_size = 0x1000usize;
            let mut remaining = count;
            let mut ptr = start.0 as usize;
            let mut out: Vec::<u8> = vec![0u8; count];
            let mut local_iovecs: Vec::<IoSliceMut> = vec![IoSliceMut::new(&mut out[..])];
            let mut remote_iovecs = Vec::<uio::RemoteIoVec>::new();
            while remaining > 0 {
                let page = min(remaining, page_size);
                remote_iovecs.push(uio::RemoteIoVec {
                    base: ptr,
                    len: page
                });
                ptr += page;
                remaining -= page;
            }
            let res = uio::process_vm_readv(
                self.pid,
                &mut local_iovecs[..],
                &remote_iovecs[..]
            );
            if let Ok(ct) = res {
                if ct > out.len() {
                    out.truncate(ct);
                }
                return Ok(out)
            }
            return error(&format!("{}", res.err().unwrap()));
        }
    }

    pub fn write_memory(&self, start: VirtAddr, data: Vec::<u8>) -> Result<()> {
        let mut remaining = data.len();
        let mut vec_idx = 0usize;
        while remaining > 0 {
            let write_sz = min(8, remaining);
            let mut bytes = [0u8; 8];
            if write_sz < 8 {
                let out = self.read_memory((start + vec_idx).into(), 8);
                if out.is_err() {
                    return error(&format!("{}", out.err().unwrap()));
                }
                &mut bytes[0..8].copy_from_slice(&out.unwrap()[0..8]);
            }
            &mut bytes[0..write_sz].copy_from_slice(&data[vec_idx..vec_idx+write_sz]);
            let res = ptrace::write(
                self.pid,
                (start + vec_idx).into(),
                i64::from_le_bytes(bytes)
            );
            if res.is_err() {
                return error(&format!("{}", res.err().unwrap()));
            }
            vec_idx += write_sz;
            remaining -= write_sz;
        }
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
                let _ = signal::kill(self.pid, signal::Signal::SIGKILL);
                let _ = wait::waitpid(self.pid, None);
            }
            let _ = ptrace::detach(self.pid, None);
            let _ = signal::kill(self.pid, signal::Signal::SIGCONT);
        }
        if self.autoterm {
            let _ = signal::kill(self.pid, signal::Signal::SIGKILL);
            let _ = wait::waitpid(self.pid, None);
        }
    }
}
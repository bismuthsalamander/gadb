#![feature(f128)]

use std::fs::read_to_string;
use std::fs;
use std::env;
use std::path::PathBuf;
use std::os::fd::AsRawFd;
use extended::Extended;

use nix::{
    unistd::Pid,
    sys::signal
};

use gadb::{
    Process,
    Result,
    error,
    Pipe,
    ValUnion,
    RegisterId,
    register_by_id,
    extend_vec
};

fn process_exists(pid: Pid) -> bool {
    let res = signal::kill(pid, None);
    res.is_ok()
}

fn get_process_status(pid: Pid) -> Result<char> {
    let Ok(procdata) = read_to_string(format!("/proc/{}/stat", pid.as_raw())) else {
        return error("could not stat process");
    };
    let Some(paren_idx) = procdata.rfind(')') else {
        return error("could not find )");
    };
    if paren_idx + 2 > procdata.len() {
        return error("could not read past )");
    }
    Ok(procdata.chars().nth(paren_idx+2).unwrap())
}

#[test]
fn launch_success() {
    let proc = Process::launch_args(&"yes", vec![String::from("--version")], true, None);
    assert!(proc.is_ok());
    assert!(process_exists(proc.unwrap().pid));
}

#[test]
fn launch_dne() {
    let proc = Process::launch(&"how_dreary_to_be_somebody");
    assert!(proc.is_err());
}

#[test]
fn attach_success() {
    let proc = Process::launch_args(env!("CARGO_BIN_EXE_quietwait"), Vec::new(), false, None);
    assert!(proc.is_ok());
    let proc = proc.unwrap();
    let proc2 = Process::attach(proc.pid.as_raw());
    assert!(proc2.is_ok());
}

#[test]
fn resume_success() {
    {
        let mut proc = Process::launch(env!("CARGO_BIN_EXE_quietwait")).unwrap();
        proc.res();
        let stat = get_process_status(proc.pid);
        assert!(stat == Ok('R') || stat == Ok('S'));
    }
    {
        let proc = Process::launch_noattach(env!("CARGO_BIN_EXE_quietwait")).unwrap();
        let proc2 = Process::attach(proc.pid.as_raw());
        assert!(proc2.is_ok());
        let mut proc2 = proc2.unwrap();
        proc2.res();
        let stat = get_process_status(proc2.pid);
        if stat != Ok('R') && stat != Ok('S') {
            dbg!(&stat);
            assert!(stat == Ok('R') || stat == Ok('S'));
        }
    }
}
#[test]
fn resume_failure() {
    let mut proc = Process::launch(env!("CARGO_BIN_EXE_fastexit")).unwrap();
    proc.res();
    let _res = proc.wait_on_signal();
    assert!(proc.resume().is_err());
}

fn get_test_binary(name: &str) -> PathBuf {
    let path_str = fs::read_to_string(
        PathBuf::from(env!("OUT_DIR")).join(name.to_owned() + "_path")
    ).expect("Failed to read reg_write path");
    
    PathBuf::from(path_str)
}

#[test]
fn write_register() {
    let test_binary = get_test_binary("reg_write");
    assert!(test_binary.exists(), "Test binary not found at {:?}", test_binary);
    
    let mut pipe = Pipe::pipe(false).unwrap();

    // Now you can use test_binary.as_path() to spawn the process
    let mut proc = Process::launch_args(&test_binary.into_os_string().into_string().unwrap(), vec![], true, Some(pipe.get_write().as_raw_fd())).unwrap();
    pipe.close_write();
    proc.resume();
    proc.wait_on_signal();
    let val: u64 = 0x1badd00d2badf00d;
    let str = format!("{:#x}", val);
    proc.write_reg(register_by_id(&RegisterId::rsi).unwrap(), val.into());
    proc.resume();
    proc.wait_on_signal();
    assert!(pipe.read_string().unwrap() == str);

    let val: u64 = 0x1fa1afe12fa1afe1;
    let str = format!("{:#x}", val);
    proc.write_reg(register_by_id(&RegisterId::mm0).unwrap(), val.into());
    proc.resume();
    proc.wait_on_signal();
    assert!(pipe.read_string().unwrap() == str);

    let val: f64 = 76.54;
    let str = format!("{0:.2}", val);
    proc.write_reg(register_by_id(&RegisterId::xmm0).unwrap(), val.into());
    proc.resume();
    proc.wait_on_signal();
    let out = pipe.read_string().unwrap();
    dbg!(&out);
    dbg!(&str);
    assert!(out == str);

    let val: f64 = 42.24;
    let str = format!("{:.2}", val);
    let val_ext: Extended = val.into();
    let val_vec16: [u8; 16] = extend_vec(val_ext.to_le_bytes());
    let res = proc.get_fpregs().unwrap();
    println!("cwd: {0:b}\nftw: {1:b}\nst0: {2:x}\nst1: {3:x}\nst2: {4:x}\nst3: {5:x}", res.swd, res.ftw, res.st_space[0], res.st_space[1], res.st_space[2], res.st_space[3]);
    proc.write_reg(register_by_id(&RegisterId::st0).unwrap(), val_vec16.into());
    let fsw: u16 = 0b0011100000000000;
    proc.write_reg(register_by_id(&RegisterId::fsw).unwrap(), fsw.into());
    let ftw: u16 = 0b0011111111111111;
    proc.write_reg(register_by_id(&RegisterId::ftw).unwrap(), ftw.into());
    let res = proc.get_fpregs().unwrap();
    println!("cwd: {0:b}\nftw: {1:b}\nst0: {2:x}\nst1: {3:x}\nst2: {4:x}\nst3: {5:x}", res.swd, res.ftw, res.st_space[0], res.st_space[1], res.st_space[2], res.st_space[3]);
    proc.resume();
    proc.wait_on_signal();
    let out = pipe.read_string().unwrap();
    dbg!(&out);
    dbg!(&str);
    assert!(out == str);
}

#[test]
fn read_register() {
    let test_binary = get_test_binary("reg_read");
    assert!(test_binary.exists(), "Test binary not found at {:?}", test_binary);
    
    let mut pipe = Pipe::pipe(false).unwrap();

    let mut proc = Process::launch_args(&test_binary.into_os_string().into_string().unwrap(), vec![], true, Some(pipe.get_write().as_raw_fd())).unwrap();
    pipe.close_write();
    proc.resume();
    proc.wait_on_signal();

    let magic_int: u64 = 0x00c0ff331deadb01;
    let magic_double: f64 = 135.79;

    assert!(proc.regs().read_as_id::<u64>(&RegisterId::r13) == magic_int);

    proc.resume();
    proc.wait_on_signal();
    assert!(proc.regs().read_as_id::<u8>(&RegisterId::r13b) == 42);

    proc.resume();
    proc.wait_on_signal();
    assert!(proc.regs().read_as_id::<u64>(&RegisterId::rbx) == 21 << 8);

    proc.resume();
    proc.wait_on_signal();
    assert!(proc.regs().read_as_id::<u64>(&RegisterId::st0) == 0xba5eba11);

    proc.resume();
    proc.wait_on_signal();
    assert!(proc.regs().read_as_id::<[u8; 8]>(&RegisterId::xmm0) == magic_double.to_le_bytes());

    proc.resume();
    proc.wait_on_signal();
    let bytes = Into::<Extended>::into(magic_double).to_le_bytes();
    let arr = proc.regs().read_as_id::<[u8; 16]>(&RegisterId::st0);
    assert!(proc.regs().read_as_id::<[u8;16]>(&RegisterId::st0)[..10] == bytes);
}
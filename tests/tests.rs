#![feature(f128)]

use std::fs::read_to_string;
use std::fs;
use std::io;
use std::io::BufReader;
use std::io::BufRead;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::os::fd::AsRawFd;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time;
use extended::Extended;

use gadb::BreakSite;
use gadb::RValue;
use gadb::VirtAddr;
use nix::{
    unistd::Pid,
    sys::signal
};

use gadb::{
    Process,
    Result,
    error,
    Pipe,
    RegisterId,
    extend_vec
};
use nix::sys::ptrace;
use regex::Regex;

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

fn re_wait(p: &mut Process) {
    &p.resume();
    &p.wait_on_signal();
}

#[test]
fn write_register() {
    let test_binary = get_test_binary("reg_write");
    assert!(test_binary.exists(), "Test binary not found at {:?}", test_binary);
    
    let mut pipe = Pipe::pipe(false).unwrap();

    // Now you can use test_binary.as_path() to spawn the process
    let mut proc = Process::launch_args(&test_binary.into_os_string().into_string().unwrap(), vec![], true, Some(pipe.get_write().as_raw_fd())).unwrap();
    pipe.close_write();
    re_wait(&mut proc);
    let val = RValue::from_id(0x1badd00d2badf00du64, RegisterId::rsi);
    let str = format!("{:#x}", val);
    proc.write_reg(&val);
        // register_by_id(&RegisterId::rsi).unwrap(), val.into());
    re_wait(&mut proc);
    assert!(pipe.read_string().unwrap() == str);

    let val = RValue::from_id(0x1fa1afe12fa1afe1u64, RegisterId::mm0);
    let str = format!("{:#x}", val.read_as::<u64>());
    proc.write_reg(&val);
    re_wait(&mut proc);
    dbg!(&str);
    dbg!(&val.ri.id);
    let out = pipe.read_string().unwrap();
    dbg!(&out);
    assert!(out == str);

    let val = RValue::from_id(76.54, RegisterId::xmm0);
    let from_val = format!("{0:.2}", val.read_as::<f64>());
    proc.write_reg(&val);
    re_wait(&mut proc);
    let from_child = pipe.read_string().unwrap();
    dbg!(&from_child);
    dbg!(&from_val);
    assert!(from_child == from_val);

    let val: f64 = 42.24;
    let str = format!("{:.2}", val);
    let val_ext: Extended = val.into();
    let val_vec16: [u8; 16] = extend_vec(val_ext.to_le_bytes());
    let res = proc.get_fpregs().unwrap();
    println!("cwd: {0:b}\nftw: {1:b}\nst0: {2:x}\nst1: {3:x}\nst2: {4:x}\nst3: {5:x}", res.swd, res.ftw, res.st_space[0], res.st_space[1], res.st_space[2], res.st_space[3]);
    proc.write_reg(&RValue::from_id(val_vec16, RegisterId::st0));
    let fsw: u16 = 0b0011100000000000;
    proc.write_reg(&RValue::from_id(fsw, RegisterId::fsw));
    let ftw: u16 = 0b0011111111111111;
    proc.write_reg(&RValue::from_id(ftw, RegisterId::ftw));
    let res = proc.get_fpregs().unwrap();
    println!("cwd: {0:b}\nftw: {1:b}\nst0: {2:x}\nst1: {3:x}\nst2: {4:x}\nst3: {5:x}", res.swd, res.ftw, res.st_space[0], res.st_space[1], res.st_space[2], res.st_space[3]);
    re_wait(&mut proc);
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
    re_wait(&mut proc);

    let magic_int: u64 = 0x00c0ff331deadb01;
    let magic_double: f64 = 135.79;

    assert!(proc.regs().read_as_id::<u64>(RegisterId::r13) == magic_int);

    re_wait(&mut proc);
    assert!(proc.regs().read_as_id::<u8>(RegisterId::r13b) == 42);

    re_wait(&mut proc);
    assert!(proc.regs().read_as_id::<u64>(RegisterId::rbx) == 21 << 8);

    re_wait(&mut proc);
    assert!(proc.regs().read_as_id::<u64>(RegisterId::st0) == 0xba5eba11);

    re_wait(&mut proc);
    assert!(proc.regs().read_as_id::<[u8; 8]>(RegisterId::xmm0) == magic_double.to_le_bytes());

    re_wait(&mut proc);
    let bytes = Into::<Extended>::into(magic_double).to_le_bytes();
    assert!(proc.regs().read_as_id::<[u8;16]>(RegisterId::st0)[..10] == bytes);
}

fn parse_hex(s: &str) -> Result<u64> {
    match u64::from_str_radix(s, 16) {
        Ok(val) => Ok(val),
        Err(_) => error("could not parse value")
    }
}
fn find_memory_offset(elf: &str, file_addr: VirtAddr) -> Result<VirtAddr> {
    // Read ELF to translate file offset to memory offset
    let mut cmd = Command::new("readelf")
        .arg("-WS")
        .arg(elf)
        .stdout(Stdio::piped())
        .spawn();
    let Ok(mut cmd) = cmd else {
        return error("could not launch readelf");
    };

    let stdout = cmd.stdout.take()
        .expect("Child process stdout handle was not configured");

    let re = Regex::new(r"PROGBITS\s+([0-9a-f]+)\s+([0-9a-f]+)\s+([0-9a-f]+)").unwrap();
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue
        };
        let Some(cap) = re.captures(&line) else {
            continue;
        };
        let (_, [addr, off, size]) = cap.extract();
        let sec_addr = parse_hex(addr)?;
        let sec_off = parse_hex(off)?;
        let sec_size = parse_hex(size)?;
        if file_addr.0 >= sec_addr && file_addr.0 < sec_addr + sec_size {
            return Ok((file_addr + sec_addr - sec_off).into())
        }
    }
    return error("could not parse readelf output");
}

fn find_memory_address(elf: &str, pid: Pid, file_addr: VirtAddr) -> Result<VirtAddr> {
    let mem_off = find_memory_offset(elf, file_addr)?;
    // Read proc map to find memory address containing this offset
    let re = Regex::new(r"^\s*([0-9a-f]+)-([0-9a-f]+)\s+..(.).\s+([0-9a-f]+)\s+").unwrap();
    let path = format!("/proc/{}/maps", pid);
    let Ok(data) = read_to_string(&path) else {
        return error("could not open procmap");
    };
    for line in data.lines() {
        let Some(cap) = re.captures(&line) else {
            continue;
        };
        let (start, end, x, offset) = (
            parse_hex(&cap[1])?,
            parse_hex(&cap[2])?,
            &cap[3],
            parse_hex(&cap[4])?
        );
        let size = end - start;
        if mem_off.0 >= offset && mem_off.0 < (offset + size) {
            return Ok((mem_off - offset) + start);
        }
    }
    return error("could not find memory address; just not found!");
}

#[test]
fn basic_breakpoints() {
    let test_binary = get_test_binary("hello_world");
    assert!(test_binary.exists(), "Test binary not found at {:?}", test_binary);

    let mut pipe = Pipe::pipe(false).unwrap();

    let mut proc = Process::launch_args(test_binary.to_str().unwrap(), vec![], true, Some(pipe.get_write().as_raw_fd())).unwrap();
    pipe.close_write();
    let bp_file_addr: VirtAddr = 0x115b.into();
    let bp_mem_addr = find_memory_address(test_binary.to_str().unwrap(), proc.pid, bp_file_addr).unwrap();
    let bpid = proc.create_breaksite(bp_mem_addr).unwrap();
    proc.enable_breaksite_by(bpid);
    re_wait(&mut proc);
    assert!(bp_mem_addr == proc.get_pc());
}

#[test]

fn memory_read() {
    let test_binary = get_test_binary("memory");
    assert!(test_binary.exists(), "Test binary not found at {:?}", test_binary);

    let mut pipe = Pipe::pipe(false).unwrap();

    let mut proc = Process::launch_args(test_binary.to_str().unwrap(), vec![], true, Some(pipe.get_write().as_raw_fd())).unwrap();
    pipe.close_write();
    re_wait(&mut proc);

    let val = 0x1badd00d2badf00du64;
    let data = pipe.read().unwrap();
    let addr: u64 = u64::from_le_bytes(data.try_into().unwrap());
    let mem = proc.read_memory(addr.into(), 8).unwrap();
    assert!(TryInto::<[u8; 8]>::try_into(mem).unwrap() == val.to_le_bytes());

    re_wait(&mut proc);
    let data = pipe.read().unwrap();
    let addr: u64 = u64::from_le_bytes(data.try_into().unwrap());
    let str = String::from("Hello, gadb");
    proc.write_memory(addr.into(), str.as_bytes().to_vec());

    re_wait(&mut proc);

    let data = pipe.read_string().unwrap();
    assert!(data == str);
}
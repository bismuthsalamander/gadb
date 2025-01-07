use std::fs::read_to_string;
use std::env;

use nix::{
    unistd::Pid,
    sys::signal
};

use gadb::{
    Process,
    Result,
    error
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
    let proc = Process::launch_args(&"yes", vec![String::from("--version")], true);
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
    let proc = Process::launch_args(env!("CARGO_BIN_EXE_quietwait"), Vec::new(), false);
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
        assert!(stat == Ok('R') || stat == Ok('S'));
    }
}
#[test]
fn resume_failure() {
    let mut proc = Process::launch(env!("CARGO_BIN_EXE_fastexit")).unwrap();
    proc.res();
    let _res = proc.wait_on_signal();
    assert!(proc.resume().is_err());
}
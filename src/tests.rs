use nix::{
    unistd::Pid,
    sys::signal
};

use crate::Process;
fn process_exists(pid: Pid) -> bool {
    let res = signal::kill(pid, None);
    res.is_ok()
}

#[test]
fn launch_success() {
    let proc = Process::launch(&"yes");
    assert!(proc.is_ok());
    assert!(process_exists(proc.unwrap().pid));
}

#[test]
fn launch_dne() {
    let proc = Process::launch(&"how_dreary_to_be_somebody");
    assert!(proc.is_err());
}
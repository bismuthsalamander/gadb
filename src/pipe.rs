use std::os::fd::{OwnedFd, RawFd, AsRawFd};

use nix::{
    unistd::{pipe2, read},
    fcntl::OFlag
};

use crate::*;

pub struct Pipe {
    fds: [Option<OwnedFd>; 2],
}

impl Pipe {
    const READ_FD: usize = 0;
    const WRITE_FD: usize = 1;

    pub fn pipe(close_on_exec: bool) -> Result<Self> {
        let out = pipe2(if close_on_exec { OFlag::O_CLOEXEC } else { OFlag::empty() });
        let Ok(out) = out else {
            return error("error creating pipe");
        };
        Ok(Self {
            fds: [Some(out.0), Some(out.1)]
        })
    }

    //TODO: error handling
    fn get_read(&self) -> RawFd {
        self.fds[Self::READ_FD].as_ref().unwrap().as_raw_fd()
    }

    //TODO: error handling
    fn get_write(&self) -> &OwnedFd {
        self.fds[Self::WRITE_FD].as_ref().unwrap()
    }

    fn _release_read(&mut self) -> RawFd {
        let ret = self.get_read();
        self.fds[Self::READ_FD] = None;
        return ret;
    }

    fn _release_write(&mut self) -> RawFd {
        let ret = self.get_write().as_raw_fd();
        self.fds[Self::WRITE_FD] = None;
        return ret;
    }

    pub fn close_read(&mut self) {
        self.fds[Self::READ_FD] = None;
    }

    pub fn close_write(&mut self) {
        self.fds[Self::WRITE_FD] = None;
    }

    pub fn read(&mut self) -> Result<Vec::<u8>> {
        let mut arr: [u8; 1024] = [0;1024];
        let res = read(self.get_read(), &mut arr);
        if res.is_err() {
            return error_os("error reading from pipe");
        }
        let sz = res.unwrap();
        Ok(arr[0..sz].to_vec())
    }

    pub fn write(&mut self, buf: Vec::<u8>) -> Result<()> {
        let res = nix::unistd::write(self.get_write(), buf.as_slice());
        if res.is_err() {
            return error_os("error writing to pipe");
        }
        Ok(())
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        self.close_read();
        self.close_write();
    }
}
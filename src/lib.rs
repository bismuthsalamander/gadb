#![feature(f128)]

mod pipe;
mod register_info;
mod registers;
mod process;
mod parsing;
mod breakpoints;
mod disassembler;

pub use {
    pipe::*,
    register_info::*,
    registers::*,
    process::*,
    parsing::*,
    breakpoints::*,
    disassembler::*
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
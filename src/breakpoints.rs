use crate::ValUnion;

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct VirtAddr (
    pub u64
);

impl std::fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "0x")?;
        }
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::UpperHex for VirtAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "0x")?;
        }
        write!(f, "{:X}", self.0)
    }
}

impl Into::<ValUnion> for VirtAddr {
    fn into(self) -> ValUnion {
        ValUnion { u64: self.0 }
    }
}

impl From::<u64> for VirtAddr {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl std::ops::Add::<u64> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Sub::<u64> for VirtAddr {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Into::<nix::sys::ptrace::AddressType> for VirtAddr {
    fn into(self) -> nix::sys::ptrace::AddressType {
        self.0 as *mut std::ffi::c_void
    }
}

impl PartialOrd for VirtAddr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for VirtAddr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

pub trait StopPoint {
    fn addr(&self) -> VirtAddr;
    fn is_at(&self, va: VirtAddr) -> bool {
        self.addr() == va
    }
    fn in_range(&self, low: VirtAddr, high: VirtAddr) -> bool {
        low.0 <= self.addr().0 && high.0 >= self.addr().0
    }
    fn set_enabled(&mut self);
    fn set_disabled(&mut self);
    fn enabled(&self) -> bool;
}

#[derive(Debug)]
pub struct BreakSite {
    pub id: usize,
    pub enabled: bool,
    pub va: VirtAddr,
    pub saved_data: Option<Vec<u8>>
}

impl BreakSite {
    pub(crate) fn new(id: usize, va: VirtAddr) -> Self {
        Self {
            id,
            enabled: true,
            va,
            saved_data: None
        }
    }
}

impl StopPoint for BreakSite {
    fn addr(&self) -> VirtAddr {
        self.va
    }
    
    fn set_enabled(&mut self) {
        self.enabled = true;
    }
    
    fn set_disabled(&mut self) {
        self.enabled = false;
    }
    
    fn enabled(&self) -> bool {
        self.enabled
    }
}
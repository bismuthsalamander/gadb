use crate::Process;
use crate::register_info::{ *, RegisterFormat::* };
use libc::user;

pub union ValUnion {
    pub u8: u8,
    pub u16: u16,
    pub u32: u32,
    pub u64: u64,
    pub i8: i8,
    pub i16: i16,
    pub i32: i32,
    pub i64: i64,
    pub f32: f32,
    pub f64: f64,
    pub f128: f128,
    pub vec8: [u8; 8],
    pub vec16: [u8; 16]
}

impl From::<u8> for ValUnion {
    fn from(value: u8) -> Self {
        Self {
            u8: value
        }
    }
}

impl From::<u16> for ValUnion {
    fn from(value: u16) -> Self {
        Self {
            u16: value
        }
    }
}

impl From::<u32> for ValUnion {
    fn from(value: u32) -> Self {
        Self {
            u32: value
        }
    }
}

impl From::<u64> for ValUnion {
    fn from(value: u64) -> Self {
        Self {
            u64: value
        }
    }
}

impl From::<i8> for ValUnion {
    fn from(value: i8) -> Self {
        Self {
            i8: value
        }
    }
}

impl From::<i16> for ValUnion {
    fn from(value: i16) -> Self {
        Self {
            i16: value
        }
    }
}

impl From::<i32> for ValUnion {
    fn from(value: i32) -> Self {
        Self {
            i32: value
        }
    }
}

impl From::<i64> for ValUnion {
    fn from(value: i64) -> Self {
        Self {
            i64: value
        }
    }
}

impl From::<f32> for ValUnion {
    fn from(value: f32) -> Self {
        Self {
            f32: value
        }
    }
}

impl From::<f64> for ValUnion {
    fn from(value: f64) -> Self {
        Self {
            f64: value
        }
    }
}

impl From::<f128> for ValUnion {
    fn from(value: f128) -> Self {
        Self {
            f128: value
        }
    }
}

impl From::<[u8; 8]> for ValUnion {
    fn from(value: [u8; 8]) -> Self {
        Self {
            vec8: value
        }
    }
}

impl From::<[u8; 16]> for ValUnion {
    fn from(value: [u8; 16]) -> Self {
        Self {
            vec16: value
        }
    }
}

struct RValue {
    val: ValUnion,
    size: usize,
    rtype: RegisterType
}

pub struct Registers {
    pub userdata: user
}

impl std::fmt::Debug for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<registers>")
    }
}

impl Registers {
    pub fn empty() -> Self {
        let v: Vec<u8> = vec![0; std::mem::size_of::<Self>()];
        let ret: *const Self;
        let ptr: *const u8 = v.as_ptr();
        unsafe {
            let ptr: *const Self = ptr as *const Self;
            ret = ptr;
            Self {
                userdata: (*ret).userdata
            }
        }
    }
    
    fn read_as<T: From<ValUnion>>(&self, ri: &RegInfo) -> T {
        let mut out = ValUnion { u8: 0 };
        unsafe {
            let ptr: *const user = &self.userdata;
            let mut ptr: *const u8 = ptr as *const u8;
            ptr = ptr.add(ri.offset);
            match (ri.format, ri.size) {
                (Uint, 1)        => { out.u8 = *ptr; },
                (Uint, 2)        => { out.u16 = *(ptr as *const u16); },
                (Uint, 4)        => { out.u32 = *(ptr as *const u32); },
                (Uint, 8)        => { out.u64 = *(ptr as *const u64); },
                (Double, 4)      => { out.f32 = *(ptr as *const f32); },
                (Double, 8)      => { out.f64 = *(ptr as *const f64); },
                (LongDouble, 16) => { out.f128 = *(ptr as *const f128); },
                (Vector, 8)      => { std::ptr::copy_nonoverlapping::<u8>(ptr, out.vec8.as_mut_ptr(), 8); },
                (Vector, 16)     => { std::ptr::copy_nonoverlapping::<u8>(ptr, out.vec16.as_mut_ptr(), 16); },
                _                => { panic!("unknown reginfo"); }
            }
        }
        out.into()
    }

    pub fn get_clong_at(&self, offset: usize) -> i64 {
        unsafe {
            let ptr: *const user = &self.userdata;
            let ptr: *const u8 = ptr as *const u8;
            let ptr = ptr.add(offset);
            let ptr: *const i64 = ptr as *const i64;
            return *ptr;
        }
    }
    pub fn write(&mut self, ri: &RegInfo, val: ValUnion) {
        unsafe {
            let mut ptr: *mut user = &mut self.userdata;
            let mut ptr: *mut u8 = ptr as *mut u8;
            ptr = ptr.add(ri.offset);
            match (ri.format, ri.size) {
                (Uint, 1)        => { *ptr = val.u8; },
                (Uint, 2)        => { *(ptr as *mut u16) = val.u16; },
                (Uint, 4)        => { *(ptr as *mut u32) = val.u32; },
                (Uint, 8)        => { *(ptr as *mut u64) = val.u64; },
                (Double, 4)      => { *(ptr as *mut f32) = val.f32; },
                (Double, 8)      => { *(ptr as *mut f64) = val.f64; },
                (LongDouble, 16) => { *(ptr as *mut f128) = val.f128; },
                (Vector, 8)      => { std::ptr::copy_nonoverlapping(val.vec8.as_ptr(), ptr, 8); },
                (Vector, 16)     => { std::ptr::copy_nonoverlapping(val.vec16.as_ptr(), ptr, 16); },
                _                => { panic!("unknown reginfo"); }
            }
        }
    }
}
// impl Registers {
//     fn read(&self, info: &RegInfo) -> 
// }
// Have a reginfo with a size field
// Want to read from registers using reference
// 
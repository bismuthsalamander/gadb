use crate::register_info::{ *, RegisterFormat::* };
use crate::error;
use crate::Result;

use extended::Extended;
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

// TODO: use a trait to restrict types that can be requested from read_as
impl ValUnion {
    pub fn format(&self, ri: &RegInfo) -> String {
        match (ri.format, ri.size) {
            (Uint, 1)        => format!("{:#x}", self.read_as::<u8>()),
            (Uint, 2)        => format!("{:#x}", self.read_as::<u16>()),
            (Uint, 4)        => format!("{:#x}", self.read_as::<u32>()),
            (Uint, 8)        => format!("{:#x}", self.read_as::<u64>()),
            (Double, 4)      => format!("{}", self.read_as::<f32>()),
            (Double, 8)      => format!("{}", self.read_as::<f64>()),
            (LongDouble, 16) => {
                // let v: [u8; 10] = self.read_as::<[u8; 16]>()[..10].into();
                let v = self.read_as::<[u8; 16]>();
                let arr: [u8; 10] = v[..10].try_into().unwrap();
                let val = Extended::from_le_bytes(arr);
                format!("{}", val.to_f64())
            },
            (Vector, 8)      => {
                let mut out = String::new();
                let v = self.read_as::<[u8; 8]>();
                for byte in v.iter() {
                    out += &format!("{:02x}", byte);
                }
                out
            },
            (Vector, 16)     => {
                let mut out = String::new();
                let v = self.read_as::<[u8; 16]>();
                for byte in v.iter() {
                    out += &format!("{:02x}", byte);
                }
                out
            },
            _ => { panic!("unknown/unsupported formt"); }
        }
    }

    fn read_as<T: 'static>(&self) -> T {
        unsafe {
            let t = TypeId::of::<T>();
            return if t == TypeId::of::<u64>() {
                (&self.u64 as *const u64 as *const T).read()
            } else if t == TypeId::of::<u32>() {
                (&self.u32 as *const u32 as *const T).read()
            } else if t == TypeId::of::<u16>() {
                (&self.u16 as *const u16 as *const T).read()
            } else if t == TypeId::of::<u8>() {
                (&self.u8 as *const u8 as *const T).read()
            } else if t == TypeId::of::<i64>() {
                (&self.i64 as *const i64 as *const T).read()
            } else if t == TypeId::of::<i32>() {
                (&self.i32 as *const i32 as *const T).read()
            } else if t == TypeId::of::<i16>() {
                (&self.i16 as *const i16 as *const T).read()
            } else if t == TypeId::of::<i8>() {
                (&self.i8 as *const i8 as *const T).read()
            } else if t == TypeId::of::<f128>() {
                (&self.f128 as *const f128 as *const T).read()
            } else if t == TypeId::of::<f64>() {
                (&self.f64 as *const f64 as *const T).read()
            } else if t == TypeId::of::<f32>() {
                (&self.f32 as *const f32 as *const T).read()
            } else if t == TypeId::of::<[u8; 8]>() {
                (&self.vec8 as *const [u8; 8] as *const T).read()
            } else if t == TypeId::of::<[u8; 16]>() {
                (&self.vec16 as *const [u8; 16] as *const T).read()
            } else {
                panic!("unknown type T");
            };
        }
    }
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

impl Into::<u8> for ValUnion {
    fn into(self) -> u8 {
        unsafe {
            self.u8
        }
    }
}

impl Into::<u16> for ValUnion {
    fn into(self) -> u16 {
        unsafe {
            self.u16
        }
    }
}

impl Into::<u32> for ValUnion {
    fn into(self) -> u32 {
        unsafe {
            self.u32
        }
    }
}

impl Into::<u64> for ValUnion {
    fn into(self) -> u64 {
        unsafe {
            self.u64
        }
    }
}

impl Into::<i8> for ValUnion {
    fn into(self) -> i8 {
        unsafe {
            self.i8
        }
    }
}

impl Into::<i16> for ValUnion {
    fn into(self) -> i16 {
        unsafe {
            self.i16
        }
    }
}

impl Into::<i32> for ValUnion {
    fn into(self) -> i32 {
        unsafe {
            self.i32
        }
    }
}

impl Into::<i64> for ValUnion {
    fn into(self) -> i64 {
        unsafe {
            self.i64
        }
    }
}

impl Into::<f32> for ValUnion {
    fn into(self) -> f32 {
        unsafe {
            self.f32
        }
    }
}

impl Into::<f64> for ValUnion {
    fn into(self) -> f64 {
        unsafe {
            self.f64
        }
    }
}

impl Into::<f128> for ValUnion {
    fn into(self) -> f128 {
        unsafe {
            self.f128
        }
    }
}

impl Into::<[u8; 8]> for ValUnion {
    fn into(self) -> [u8; 8] {
        unsafe {
            self.vec8
        }
    }
}

impl Into::<[u8; 16]> for ValUnion {
    fn into(self) -> [u8; 16] {
        unsafe {
            self.vec16
        }
    }
}

struct _RValue {
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

pub fn extend_vec<const L: usize>(val: [u8; L]) -> [u8; 16] {
    assert!(L <= 16);
    let mut out = [0; 16];
    out[..L].copy_from_slice(&val);
    out
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
    
    pub fn read_as_id<T: 'static>(&self, rid: &RegisterId) -> T {
        let ri = register_by_id(rid).unwrap();
        self.read_as(ri)
    }

    pub fn read_as<T: 'static>(&self, ri: &RegInfo) -> T {
        let out = self.read(ri);
        out.read_as()
    }

    pub fn read(&self, ri: &RegInfo) -> ValUnion {
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
        out
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
            let mut ptr: *mut u8 = &mut self.userdata as *mut user as *mut u8;
            ptr = ptr.add(ri.offset);
            match (ri.format, ri.size) {
                (Uint, 1)        => { *ptr = val.u8; },
                (Uint, 2)        => { *(ptr as *mut u16) = val.u16; },
                (Uint, 4)        => { *(ptr as *mut u32) = val.u32; },
                (Uint, 8)        => { *(ptr as *mut u64) = val.u64; },
                (Double, 4)      => { *(ptr as *mut f32) = val.f32; },
                (Double, 8)      => { *(ptr as *mut f64) = val.f64; },
                // TODO: fix this! we want f80/Extended
                (LongDouble, 16) => { *(ptr as *mut f128) = val.f128; },
                (Vector, 8)      => { std::ptr::copy_nonoverlapping(val.vec8.as_ptr(), ptr, 8); },
                (Vector, 16)     => { std::ptr::copy_nonoverlapping(val.vec16.as_ptr(), ptr, 16); },
                _                => { panic!("unknown reginfo"); }
            }
        }
    }
}
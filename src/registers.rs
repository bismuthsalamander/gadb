use crate::register_info::{ *, RegisterFormat::* };

use extended::Extended;
use libc::user;

use std::any::TypeId;

pub trait RegType where Self: PartialEq, Self: Copy, Self: 'static {}
impl RegType for u8 {}
impl RegType for u16 {}
impl RegType for u32 {}
impl RegType for u64 {}
impl RegType for i8 {}
impl RegType for i16 {}
impl RegType for i32 {}
impl RegType for i64 {}
impl RegType for f32 {}
impl RegType for f64 {}
impl RegType for [u8; 8] {}
impl RegType for [u8; 16] {}

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
    pub vec8: [u8; 8],
    pub vec16: [u8; 16]
}

impl ValUnion {
    fn from<T: RegType>(t: T) -> Self {
        let mut out = Self { u8: 0 };
        unsafe {
            std::ptr::copy_nonoverlapping(&t, (&mut out.u8 as *mut u8) as *mut T, 1);
        }
        out
    }
}
pub struct RValue {
    pub val: ValUnion,
    pub ri: &'static RegInfo
}

impl<T: RegType> PartialEq<T> for RValue {
    fn eq(&self, other: &T) -> bool {
        self.read_as::<T>() == *other
    }
}

impl std::fmt::Display for RValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.ri.format, self.ri.size) {
            (Uint, _)        => write!(f, "{:#x}", unsafe { self.val.u64 } ),
            (Double, 4)      => write!(f, "{}", unsafe { self.val.f32 } ),
            (Double, 8)      => write!(f, "{}", unsafe { self.val.f64 } ),
            (LongDouble, 16) => {
                let v = unsafe { &self.val.vec16 };
                let arr: [u8; 10] = v[..10].try_into().unwrap();
                let val = Extended::from_le_bytes(arr);
                write!(f, "{}", val.to_f64())
            },
            (Vector, 8)      => {
                let v = unsafe { &self.val.vec8 };
                for byte in v.iter() {
                    write!(f, "{:02x}", byte)?;
                }
                Ok(())
            },
            (Vector, 16)     => {
                let v = unsafe { &self.val.vec16 };
                for byte in v.iter() {
                    write!(f, "{:02x}", byte)?;
                }
                Ok(())
            },
            _ => { panic!("unknown/unsupported formt"); }
        }
    }
}

impl std::fmt::LowerHex for RValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.ri.format, self.ri.size) {
            (Uint, _)        => {
                if f.alternate() {
                    write!(f, "0x")?;
                }
                write!(f, "{:x}", unsafe { self.val.u64 } )
            },
            (Double, 4)      => {
                for val in unsafe { self.val.f32.to_le_bytes() } {
                    write!(f, "{:x}", val)?;
                }
                Ok(())
            },
            (Double, 8)      => {
                for val in unsafe { self.val.f64.to_le_bytes() } {
                    write!(f, "{:x}", val)?;
                }
                Ok(())
            }
            (LongDouble, 16) => {
                let v = unsafe { &self.val.vec16 };
                let arr: [u8; 10] = v[..10].try_into().unwrap();
                for val in arr.iter() {
                    write!(f, "{}", val)?;
                }
                Ok(())
            },
            (Vector, 8)      => {
                write!(f, "[")?;
                let v = unsafe { &self.val.vec8 };
                for (idx, byte) in v.iter().enumerate() {
                    write!(f, "{:02x}", byte)?;
                    if idx != v.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            },
            (Vector, 16)     => {
                write!(f, "[")?;
                let v = unsafe { &self.val.vec8 };
                for (idx, byte) in v.iter().enumerate() {
                    write!(f, "{:02x}", byte)?;
                    if idx != v.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            },
            _ => { panic!("unknown/unsupported format"); }
        }
    }
}

impl RValue {
    pub fn from_id<T: RegType>(t: T, rid: RegisterId) -> Self {
        Self {
            val: ValUnion::from(t),
            ri: register_by_id(rid).unwrap()
        }
    }

    pub fn from<T: RegType>(t: T, ri: &'static RegInfo) -> Self {
        Self {
            val: ValUnion::from(t),
            ri
        }
    }

    pub fn read_as<T: RegType>(&self) -> T {
        unsafe {
            let t = TypeId::of::<T>();
            return if t == TypeId::of::<u64>() {
                (&self.val.u64 as *const u64 as *const T).read().clone()
            } else if t == TypeId::of::<u32>() {
                (&self.val.u32 as *const u32 as *const T).read().clone()
            } else if t == TypeId::of::<u16>() {
                (&self.val.u16 as *const u16 as *const T).read().clone()
            } else if t == TypeId::of::<u8>() {
                (&self.val.u8 as *const u8 as *const T).read().clone()
            } else if t == TypeId::of::<i64>() {
                (&self.val.i64 as *const i64 as *const T).read().clone()
            } else if t == TypeId::of::<i32>() {
                (&self.val.i32 as *const i32 as *const T).read().clone()
            } else if t == TypeId::of::<i16>() {
                (&self.val.i16 as *const i16 as *const T).read().clone()
            } else if t == TypeId::of::<i8>() {
                (&self.val.i8 as *const i8 as *const T).read().clone()
            } else if t == TypeId::of::<f64>() {
                (&self.val.f64 as *const f64 as *const T).read().clone()
            } else if t == TypeId::of::<f32>() {
                (&self.val.f32 as *const f32 as *const T).read().clone()
            } else if t == TypeId::of::<[u8; 8]>() {
                (&self.val.vec8 as *const [u8; 8] as *const T).read().clone()
            } else if t == TypeId::of::<[u8; 16]>() {
                (&self.val.vec16 as *const [u8; 16] as *const T).read().clone()
            } else {
                panic!("unknown type T");
            };
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
    
    pub fn read_as_id<T: RegType>(&self, rid: RegisterId) -> T {
        let ri = register_by_id(rid).unwrap();
        self.read_as(ri)
    }

    pub fn read_as<T: RegType>(&self, ri: &'static RegInfo) -> T {
        let out = self.read(ri);
        out.read_as()
    }

    pub fn read(&self, ri: &'static RegInfo) -> RValue {
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
                (LongDouble, 16) => { std::ptr::copy_nonoverlapping::<u8>(ptr, out.vec16.as_mut_ptr(), 16); },
                (Vector, 8)      => { std::ptr::copy_nonoverlapping::<u8>(ptr, out.vec8.as_mut_ptr(), 8); },
                (Vector, 16)     => { std::ptr::copy_nonoverlapping::<u8>(ptr, out.vec16.as_mut_ptr(), 16); },
                _                => { panic!("unknown reginfo"); }
            }
        }
        RValue {
            val: out,
            ri: ri
        }
    }

    pub fn get_clong_at(&self, offset: usize) -> i64 {
        unsafe {
            let mut ptr: *const u8 = (&self.userdata as *const user) as *const u8;
            ptr = ptr.add(offset);
            let ptr: *const i64 = ptr as *const i64;
            return *ptr;
        }
    }

    pub fn write(&mut self, rv: &RValue) {
        unsafe {
            let mut ptr: *mut u8 = &mut self.userdata as *mut user as *mut u8;
            ptr = ptr.add(rv.ri.offset);
            match (rv.ri.format, rv.ri.size) {
                (Uint, 1)        => { *ptr = rv.val.u8; },
                (Uint, 2)        => { *(ptr as *mut u16) = rv.val.u16; },
                (Uint, 4)        => { *(ptr as *mut u32) = rv.val.u32; },
                (Uint, 8)        => { *(ptr as *mut u64) = rv.val.u64; },
                (Double, 4)      => { *(ptr as *mut f32) = rv.val.f32; },
                (Double, 8)      => { *(ptr as *mut f64) = rv.val.f64; },
                (LongDouble, 16) => { std::ptr::copy_nonoverlapping(rv.val.vec16.as_ptr(), ptr, 16); }
                (Vector, 8)      => { std::ptr::copy_nonoverlapping(rv.val.vec8.as_ptr(), ptr, 8); },
                (Vector, 16)     => { std::ptr::copy_nonoverlapping(rv.val.vec16.as_ptr(), ptr, 16); },
                _                => { panic!("unknown reginfo"); }
            }
        }
    }
}
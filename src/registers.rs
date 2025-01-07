use libc::{
    user,
    user_regs_struct,
    user_fpregs_struct
};

use std::mem;

#[allow(non_camel_case_types)]
enum RegisterId {
    rax,
    rbx,
    rcx,
    rdx,
    rsi,
    rdi,
    rbp,
    rsp,
    r8,
    r9,
    r10,
    r11,
    r12,
    r13,
    r14,
    r15,
    rip,
    eflags,
    cs,
    fs,
    gs,
    ss,
    ds,
    es,
    orig_rax,

    eax,
	edx,
    ecx,
	ebx,
    esi,
	edi,
    ebp,
	esp,
    r8d,
	r9d,
    r10d,
	r11d,
    r12d,
	r13d,
    r14d,
	r15d,

    ax,
	dx,
    cx,
	bx,
    si,
	di,
    bp,
	sp,
    r8w,
	r9w,
    r10w,
	r11w,
    r12w,
	r13w,
    r14w,
	r15w,
    ah,
	dh,
    ch,
	bh,
    al,
	dl,
    cl,
	bl,
    sil,
	dil,
    bpl,
	spl,
    r8b,
	r9b,
    r10b,
	r11b,
    r12b,
	r13b,
    r14b,
	r15b, 

    fcw,
    fsw,
    ftw,
    fop,
    frip,
    frdp,
    mxcsr,
    mxcsrmask,

    // fpr!(st0, St0, 33, 16)
    // ( $num:expr, $name:expr, $id:ident, $dwarf_id: expr )

    st0,
    st1,
    st2,
    st3,
    st4,
    st5,
    st6,
    st7,

    mm0,
    mm1,
    mm2,
    mm3,
    mm4,
    mm5,
    mm6,
    mm7,

    xmm0,
    xmm1,
    xmm2,
    xmm3,
    xmm4,
    xmm5,
    xmm6,
    xmm7,
    xmm8,
    xmm9,
    xmm10,
    xmm11,
    xmm12,
    xmm13,
    xmm14,
    xmm15,

    dr0,
    dr1,
    dr2,
    dr3,
    dr4,
    dr5,
    dr6,
    dr7,
}

#[derive(Clone, Copy)]
enum RegisterType {
    Gpr,
    SubGpr,
    Fpr,
    Dr
}

use RegisterType::*;

#[derive(Clone, Copy)]
enum RegisterFormat {
    Uint,
    Double,
    LongDouble,
    Vector
}

use RegisterFormat::*;

struct RegInfo {
    name: &'static str,
    id: RegisterId,
    rtype: RegisterType,
    format: RegisterFormat,
    dwarf_id: i32,
    size: usize,
    offset: usize,
}

macro_rules! reg {
    ( $name:ident, $rtype:expr, $format:expr, $dwarf_id:expr, $size:expr, $offset:expr ) => {
        RegInfo {
            name: stringify!($name),
            id: RegisterId::$name,
            rtype: $rtype,
            format: $format,
            dwarf_id: $dwarf_id,
            size: $size,
            offset: $offset
        }
    }
}

macro_rules! gpr64 {
    ( $name:ident, $dwarf_id:expr ) => {
        reg!($name, Gpr, Uint, $dwarf_id, 8, mem::offset_of!(libc::user, regs) + mem::offset_of!(libc::user_regs_struct, $name))
    };
}

macro_rules! gpr32 {
    ( $name:ident, $parent:ident ) => {
        reg!($name, SubGpr, Uint, -1, 4, mem::offset_of!(libc::user, regs) + mem::offset_of!(libc::user_regs_struct, $parent))
    };
}

macro_rules! gpr16 {
    ( $name:ident, $parent:ident ) => {
        reg!($name, SubGpr, Uint, -1, 2, mem::offset_of!(libc::user, regs) + mem::offset_of!(libc::user_regs_struct, $parent))
    };
}

macro_rules! gpr8h {
    ( $name:ident, $parent:ident ) => {
        reg!($name, SubGpr, Uint, -1, 1, mem::offset_of!(libc::user, regs) + mem::offset_of!(libc::user_regs_struct, $parent) + 1)
    };
}

macro_rules! gpr8l {
    ( $name:ident, $parent:ident ) => {
        reg!($name, SubGpr, Uint, -1, 1, mem::offset_of!(libc::user, regs) + mem::offset_of!(libc::user_regs_struct, $parent))
    };
}

macro_rules! fpr {
    ( $name:ident, $dwarf_id: expr, $size:expr, $libc_name:ident ) => {
        reg!($name, Fpr, Uint, $dwarf_id, $size, mem::offset_of!(libc::user, i387) + mem::offset_of!(libc::user_fpregs_struct, $libc_name))
    }
}

macro_rules! fpr_st {
    ( $num:expr, $name:ident ) => {
        reg!($name, Fpr, LongDouble, 33 + $num, 16, mem::offset_of!(libc::user, i387) + mem::offset_of!(libc::user_fpregs_struct, st_space) + (16 * $num))
    }
}

macro_rules! fpr_mm {
    ( $num:expr, $name:ident ) => {
        reg!($name, Fpr, Vector, 41 + $num, 8, mem::offset_of!(libc::user, i387) + mem::offset_of!(libc::user_fpregs_struct, st_space) + (16 * $num))
    }
}

macro_rules! fpr_xmm {
    ( $num:expr, $name:ident ) => {
        reg!($name, Fpr, Vector, 17 + $num, 16, mem::offset_of!(libc::user, i387) + mem::offset_of!(libc::user_fpregs_struct, xmm_space) + (16 * $num))
    }
}

macro_rules! dr {
    ( $num:expr, $name:ident ) => {
        reg!($name, Dr, Uint, -1, 8, mem::offset_of!(libc::user, u_debugreg) + (8 * $num))
    }
}

const REGISTER_INFOS: [RegInfo; 125] = [
    gpr64!(rax, 0),
    gpr64!(rdx, 1),
    gpr64!(rcx, 2),
    gpr64!(rbx, 3),
    gpr64!(rsi, 4),
    gpr64!(rdi, 5),
    gpr64!(rbp, 6),
    gpr64!(rsp, 7),
    gpr64!(r8, 8),
    gpr64!(r9, 9),
    gpr64!(r10, 10),
    gpr64!(r11, 11),
    gpr64!(r12, 12),
    gpr64!(r13, 13),
    gpr64!(r14, 14),
    gpr64!(r15, 15),
    gpr64!(rip, 16),
    gpr64!(eflags, 49),
    gpr64!(cs, 51),
    gpr64!(fs, 54),
    gpr64!(gs, 55),
    gpr64!(ss, 52),
    gpr64!(ds, 53),
    gpr64!(es, 50),

    gpr64!(orig_rax, -1),

    gpr32!(eax, rax),
	gpr32!(edx, rdx),
    gpr32!(ecx, rcx),
	gpr32!(ebx, rbx),
    gpr32!(esi, rsi),
	gpr32!(edi, rdi),
    gpr32!(ebp, rbp),
	gpr32!(esp, rsp),
    gpr32!(r8d, r8),
	gpr32!(r9d, r9),
    gpr32!(r10d, r10),
	gpr32!(r11d, r11),
    gpr32!(r12d, r12),
	gpr32!(r13d, r13),
    gpr32!(r14d, r14),
	gpr32!(r15d, r15),

    gpr16!(ax, rax),
	gpr16!(dx, rdx),
    gpr16!(cx, rcx),
	gpr16!(bx, rbx),
    gpr16!(si, rsi),
	gpr16!(di, rdi),
    gpr16!(bp, rbp),
	gpr16!(sp, rsp),
    gpr16!(r8w, r8),
	gpr16!(r9w, r9),
    gpr16!(r10w, r10),
	gpr16!(r11w, r11),
    gpr16!(r12w, r12),
	gpr16!(r13w, r13),
    gpr16!(r14w, r14),
	gpr16!(r15w, r15),

    gpr8h!(ah, rax),
	gpr8h!(dh, rdx),
    gpr8h!(ch, rcx),
	gpr8h!(bh, rbx),

    gpr8l!(al, rax),
	gpr8l!(dl, rdx),
    gpr8l!(cl, rcx),
	gpr8l!(bl, rbx),
    gpr8l!(sil, rsi),
	gpr8l!(dil, rdi),
    gpr8l!(bpl, rbp),
	gpr8l!(spl, rsp),
    gpr8l!(r8b, r8),
	gpr8l!(r9b, r9),
    gpr8l!(r10b, r10),
	gpr8l!(r11b, r11),
    gpr8l!(r12b, r12),
	gpr8l!(r13b, r13),
    gpr8l!(r14b, r14),
	gpr8l!(r15b, r15),

    //( $name:expr, $id:ident, $dwarf_id: expr, $size:expr, $libc_name:expr )
    fpr!(fcw, 65, 2, cwd),
    fpr!(fsw, 66, 2, swd),
    fpr!(ftw, -1, 2, ftw),
    fpr!(fop, -1, 2, fop),
    fpr!(frip, -1, 8, rip),
    fpr!(frdp, -1, 8, rdp),
    fpr!(mxcsr, 64, 4, mxcsr),
    fpr!(mxcsrmask, -1, 4, mxcr_mask),

    // fpr!(st0, St0, 33, 16)
    // ( $num:expr, $name:expr, $id:ident, $dwarf_id: expr )

    fpr_st!(0, st0),
    fpr_st!(1, st1),
    fpr_st!(2, st2),
    fpr_st!(3, st3),
    fpr_st!(4, st4),
    fpr_st!(5, st5),
    fpr_st!(6, st6),
    fpr_st!(7, st7),

    fpr_mm!(0, mm0),
    fpr_mm!(1, mm1),
    fpr_mm!(2, mm2),
    fpr_mm!(3, mm3),
    fpr_mm!(4, mm4),
    fpr_mm!(5, mm5),
    fpr_mm!(6, mm6),
    fpr_mm!(7, mm7),

    fpr_xmm!(0, xmm0),
    fpr_xmm!(1, xmm1),
    fpr_xmm!(2, xmm2),
    fpr_xmm!(3, xmm3),
    fpr_xmm!(4, xmm4),
    fpr_xmm!(5, xmm5),
    fpr_xmm!(6, xmm6),
    fpr_xmm!(7, xmm7),
    fpr_xmm!(8, xmm8),
    fpr_xmm!(9, xmm9),
    fpr_xmm!(10, xmm10),
    fpr_xmm!(11, xmm11),
    fpr_xmm!(12, xmm12),
    fpr_xmm!(13, xmm13),
    fpr_xmm!(14, xmm14),
    fpr_xmm!(15, xmm15),

    dr!(0, dr0),
    dr!(1, dr1),
    dr!(2, dr2),
    dr!(3, dr3),
    dr!(4, dr4),
    dr!(5, dr5),
    dr!(6, dr6),
    dr!(7, dr7),
];
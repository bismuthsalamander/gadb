use crate::{
    VirtAddr,
    Process,
    Result,
    error
};
use iced_x86::{
    Decoder, GasFormatter, Instruction as II, Formatter,
    Mnemonic
};

pub struct Instruction {
    pub va: VirtAddr,
    pub text: String,
    pub opcodes: Vec::<u8>
}

pub fn disassemble(p: &Process, rip: VirtAddr, mut max_inst: Option<usize>) -> Result<Vec<Instruction>> {
    let max_inst = max_inst.unwrap_or(30);
    let mut buf = match p.read_memory_clean(rip, max_inst * 15) {
        Ok(buf) => buf,
        Err(e) => return error(&e.msg)
    };
    let mut out = Vec::new();
    let mut inst_left = max_inst;
    let mut ip_offset: u64 = 0;

    let mut fmt_output = String::new();
    let mut decoder = Decoder::with_ip(64, &buf[..], rip.0, 0);
    let mut instr_buf = II::default();

    let mut formatter = GasFormatter::new();
    while inst_left > 0 {
        let va: VirtAddr = decoder.ip().into();
        let buf_start = (va - rip) as usize;
        decoder.decode_out(&mut instr_buf);
        if instr_buf.is_invalid() {
            let mut buf_new = match p.read_memory_clean(rip, 1024) {
                Ok(b) => b,
                Err(e) => return error(&e.msg)
            };
            buf.extend(buf_new.iter());
            decoder = Decoder::with_ip(64, &buf[buf_start..], va.0, 0);
            continue;
        }
        fmt_output.clear();
        formatter.format(&instr_buf, &mut fmt_output);
        out.push(Instruction {
            va,
            text: fmt_output.clone(),
            opcodes: buf[buf_start..buf_start+instr_buf.len()].to_vec()
        });
        inst_left -= 1;
        if instr_buf.mnemonic() == Mnemonic::Ret {
            break;
        }
    }
    Ok(out)
}
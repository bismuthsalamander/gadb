use std::cmp::min;

use gadb::{parse_hex_vec, StopPoint};
use copperline::Copperline;
use gadb::{
    error, parse_float, parse_u64, parse_vec, register_by_name, Process, RValue, RegisterFormat, RegisterType, Result, REGISTER_INFOS
};

fn usage() {
    let args: Vec<String> = std::env::args().collect();
    eprintln!("usage: {} -p [pid]\n       {} [path]", &args[0], &args[0]);
}

fn attach(args: &Vec::<String>) -> Result<Process> {
    let res: Result<Process>;
    if args.len() == 3 && args[1] == "-p" {
        let Ok(pid) = args[2].parse::<i32>() else {
            panic!("invalid pid: {}", args[2]);
        };
        res = Process::attach(pid);
    } else {
        res = Process::launch_args(
            &args[1],
            (2..args.len()).map(|idx| args[idx].clone()).collect(),
            true,
            None
        );
    }
    res
}

fn print_help(args: &Vec<&str>) {
    if args.len() <= 1 {
        println!("Available comamnds:

    continue
    memory
    register
    breakpoint");
    } else {
        if "register".starts_with(args[0]) {
            println!("Usage: register (subcommand)

Available subcommands:

    read <register>
    read all
    write <register> <value>");
        } else if "breakpoint".starts_with(args[0]) {
            println!("Usage: breakpoint (subcommand)

Available subcommands:

    list
    set <addr>
    enable <addr|id>
    disable <addr|id>
    clear <addr|id>");
        } else if "memory".starts_with(args[0]) {
            println!("Usage: memory (subcommand)

Available subcommands:

    read <addr> <numbytes>
    write <addr> <data>");
        }
    }
}

fn handle_memory_command(p: &mut Process, args: &Vec<&str>) {
    if args.len() < 3 {
        return print_help(args);
    }
    if "read".starts_with(args[1]) {
        let numbytes: usize = if args.len() == 3 {
            32
        } else {
            match parse_u64(args[3]) {
                Ok(n) => n as usize,
                Err(e) => {
                    println!("{}", e);
                    return;
                },
            }
        };
        let Ok(addr) = parse_u64(args[2]) else {
            println!("could not parse address");
            return;
        };

        let data = p.read_memory(addr.into(), numbytes);
        match data {
            Ok(data) => {
                let page_size = 16;
                let mut remaining = data.len();
                let mut idx: usize = 0;
                while remaining > 0 {
                    print!("{:#016x}:", addr + idx as u64);
                    let sz = min(page_size, remaining);
                    for val in &data[idx..idx+sz] {
                        print!(" {:02x}", val);
                    }
                    print!("\n");
                    remaining -= sz;
                    idx += sz;
                }
            },
            Err(e) => println!("{}", e),
        }
        return;
    } else if "write".starts_with(args[1]) {
        let Ok(addr) = parse_u64(args[2]) else {
            println!("could not parse address");
            return;
        };

        let mut bytes = parse_hex_vec(args[3]);
        let bytes = match bytes {
            Ok(b) => b,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        
        let res = p.write_memory(addr.into(), bytes);
        if res.is_err() {
            println!("{}", res.err().unwrap());
        }
        return;
    }
}

fn handle_register_command(p: &mut Process, args: &Vec<&str>) {
    if args.len() < 2 {
        return print_help(args);
    }
    if "read".starts_with(args[1]) {
        if args.len() == 2 || "all".starts_with(args[2]) {
            for ri in REGISTER_INFOS.iter() {
                if ri.rtype != RegisterType::Gpr || ri.dwarf_id == -1 {
                    continue;
                }
                let val = p.regs().read(ri);
                println!("{}:\t{}", ri.name, val);
            }
        } else {
            let Ok(ri) = register_by_name(args[2]) else {
                return println!("Unrecognized register {}", args[2]);
            };
            let val = p.regs().read(ri);
            println!("{}:\t{}", ri.name, val);
        }
    } else if "write".starts_with(args[1]) {
        if args.len() != 4 {
            return print_help(args);
        }
        let Ok(ri) = register_by_name(args[2]) else {
            return println!("Unrecognized register {}", args[2]);
        };

        //TODO: move this to parsing?
        let val = match ri.format {
            RegisterFormat::Uint => {
                let Ok(v) = parse_u64(&args[3]) else {
                    return println!("could not parse value");
                };
                RValue::from(v, ri)
            },
            RegisterFormat::Double => {
                let Ok(v) = parse_float(&args[3]) else {
                    return println!("could not parse value");
                };
                RValue::from(v, ri)
            },
            RegisterFormat::LongDouble => {
                return println!("not supported yet");
            },
            RegisterFormat::Vector => {
                if ri.size == 8 {
                    let Ok(v) = parse_vec::<8>(&args[3]) else {
                        return println!("could not parse value");
                    };
                    RValue::from(v, ri)
                } else {
                    let Ok(v) = parse_vec::<16>(&args[3]) else {
                        return println!("could not parse value");
                    };
                    RValue::from(v, ri)
                }
                
            },
        };
        p.write_reg(&val);
    }
}

fn handle_breakpoint_command(p: &mut Process, args: &Vec<&str>) {
    let max_id = p.breaksites().iter().map(|b| b.id).max().unwrap_or(0);
    if args.len() == 2 {
        if let Ok(_) = parse_u64(args[1]) {
            return handle_breakpoint_command(p, &vec![args[0], "set", args[1]]);
        }
    }
    if "list".starts_with(args[1]) || "show".starts_with(args[1]) {
        let mut bs = p.breaksites();
        if bs.len() == 0 {
            println!("No breakpoints created");
            return;
        }
        bs.sort_by_key(|k| k.addr());
        let top_id = bs.iter().map(|b| b.id).max().unwrap();
        let len = format!("{}", top_id).len();
        println!("Breakpoints:");
        for bp in bs {
            println!("{:>len$}:\t{:#x}", bp.id, bp.addr());
        }
    } else if args.len() < 3 {
        return print_help(args);
    } else if "set".starts_with(args[1]) {
        let Ok(val) = parse_u64(args[2]) else {
            println!("could not parse address");
            return;
        };
        let res = p.create_breaksite(val.into());
        let Ok(id) = res else {
            println!("{}", res.err().unwrap());
            return;
        };
        let res = p.enable_breaksite_by(id);
        let Ok(_) = res else {
            println!("{}", res.err().unwrap());
            return;
        };
        println!("created breaksite {}", id);
    } else if "enable".starts_with(args[1]) || "disable".starts_with(args[1]) {
        let enable = "enable".starts_with(args[1]);
        if args[2] == "all" {
            if enable {
                p.enable_all_breaksites();
            } else {
                p.disable_all_breaksites();
            }
            return;
        }

        let Ok(val) = parse_u64(args[2]) else {
            println!("could not parse address or ID");
            return;
        };
        let id = {
            let bs = if val as usize > max_id {
                p.breaksite_at_va(val.into())
            } else {
                p.breaksite_by_id(val as usize)
            };
            let Some(bs) = bs else {
                println!("could not find specified breakpoint");
                return;
            };
            if enable == bs.enabled() {
                println!("breaksite already {}abled", if enable { "en" } else { "dis" });
                return;
            }
            bs.id
        };
        if enable {
            match p.enable_breaksite_by(id) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("breakpoint {} enabled", id)
            }
        } else {
            match p.disable_breaksite_by(id) {
                Err(e) => println!("{}", e),
                Ok(_) => println!("breakpoint {} disabled", id),
            }
        }
    } else if "clear".starts_with(args[1]) {
        if args[2] == "all" {
            p.clear_all_breaksites();
            return;
        }
        let Ok(val) = parse_u64(args[2]) else {
            println!("could not parse address or ID");
            return;
        };
        let id = {
            let Some(bs) = (if val > max_id as u64 {
                p.breaksite_at_va(val.into())
            } else {
                p.breaksite_by_id(val as usize)
            }) else {
                println!("could not find breakpoint");
                return;
            };
            bs.id
        };
        let res = p.clear_breaksite(id);
        if res.is_err() {
            println!("{}", res.err().unwrap());
        }
    }
}

fn handle_command(p: &mut Process, cmd: &str) -> Result<()> {
    let split = cmd.split(' ');
    let args: Vec<&str> = split.collect();
    let command = args.get(0);
    let Some(command) = command else {
        return error("could not read command");
    };
    if "continue".starts_with(command) {
        p.resume()?;
        let reason = p.wait_on_signal();
        if let Ok(reason) = reason {
            println!("{}", &reason);
        } else {
            return Err(reason.err().unwrap());
        }
    } else if "help".starts_with(command) {
        print_help(&args);
    } else if "registers".starts_with(command) {
        handle_register_command(p, &args);
    } else if "breakpoint".starts_with(command) {
        handle_breakpoint_command(p, &args);
    } else if "memory".starts_with(command) {
        handle_memory_command(p, &args);
    } else {
        return error(&format!("unrecognized command: {}", command));
    }
    Ok(())
}

fn main_loop(mut p: Process, mut cl: Copperline) {
    loop {
        let line = cl.read_line_ascii("gadb> ");
        let Ok(line) = line else {
            return;
        };
        let mut exec_line: &str = &"";
        if line == "" {
            if cl.get_current_history_length() > 0 {
                let h = cl.get_history_item(cl.get_current_history_length() - 1);
                if h.is_some() {
                    exec_line = &h.unwrap();
                }
            }
        } else {
            exec_line = &line;
            cl.add_history(line.clone());
        }
        if !exec_line.is_empty() {
            let res = handle_command(&mut p, exec_line);
            if res.is_err() {
                println!("{}", res.err().unwrap());
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        usage();
        return;
    }
    let process = attach(&args);
    let Ok(process) = process else {
        println!("{}", process.err().unwrap());
        return;
    };
    println!("pid: {}", process.pid.as_raw());
    main_loop(process, Copperline::new());
}

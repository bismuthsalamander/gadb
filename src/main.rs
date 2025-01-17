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
    register
");
    } else {
        if "register".starts_with(args[1]) {
            println!("Usage: register (subcommand)

Available subcommands:

    read
    read <register>
    read all
    write <register> <value>");
        }
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


        // TODO: this repetition will all go away after I fix how ValUnion is used
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

use copperline::Copperline;
use gadb::{
    Process,
    Result,
    error
};

fn usage() {
    let args: Vec<String> = std::env::args().collect();
    eprintln!("usage: {} -p [pid]\n       {} [path]", &args[0], &args[0]);
}

fn attach(args: &Vec::<String>) -> Process {
    let res: Result<Process>;
    if args.len() == 3 && args[1] == "-p" {
        let Ok(pid) = args[2].parse::<i32>() else {
            panic!("invalid pid: {}", args[2]);
        };
        res = Process::attach(pid);
    } else {
        res = Process::launch(&args[1]);
    }
    if res.is_err() {
        panic!("error attaching: {}", res.err().unwrap());
    }
    return res.unwrap();
}

// TODO: error type
fn handle_command(p: &mut Process, cmd: &str) -> Result<()> {
    let mut split = cmd.split(' ');
    let command = split.nth(0);
    let Some(command) = command else {
        return error("could not read command");
    };
    if "continue".starts_with(command) {
        p.resume();
        let reason = p.wait_on_signal();
        println!("{}", &reason);
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
    println!("pid: {}", process.pid.as_raw());
    let mut cl = Copperline::new();
    main_loop(process, cl);
}

use regex::Regex;
use std::env::{current_exe, Args};
use std::fs::{create_dir, File};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

mod refresh;

enum Command {
    Add(Task),
    Remove(String),
    List(Option<String>),
    Refresh,
    Clear,
    Help,
}

pub struct Task {
    data: [String; 4],
    auto_delete: bool,
}

impl Task {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for field in &self.data {
            string.push_str(&field);
            string.push('|');
        }
        if self.auto_delete {
            string.push('1');
        } else {
            string.push('0');
        }
        string.push('\n');
        return string;
    }
}

struct TaskFile {
    path: PathBuf,
}

impl TaskFile {
    fn new(path: PathBuf) -> TaskFile {
        return TaskFile { path };
    }

    fn add_task(&self, task: Task) -> Result<(), String> {
        let tasks = self.parse()?;
        let mut new_tasks = String::new();
        let mut completed = false;
        let priority: u32 = handle(
            task.data[3].parse(),
            "Priority must be a number. See `todo --help` for usage.",
        )?;
        for t in tasks {
            if t.data[0] == task.data[0] {
                return Err(format!("Duplicate task '{}'.", t.data[0]));
            }
            if !completed
                && handle(
                    t.data[3].parse::<u32>(),
                    "Previous task has non integer priority.",
                )? < priority
            {
                new_tasks.push_str(&task.to_string());
                completed = true;
            }
            new_tasks.push_str(&t.to_string());
        }
        if !completed {
            new_tasks.push_str(&task.to_string());
        }
        return self.write(new_tasks);
    }

    fn remove_task(&mut self, re: String) -> Result<(), String> {
        let tasks = self.parse()?;
        let re = handle(
            Regex::new(&re),
            "Invalid regular expression. See `todo --help` for usage.",
        )?;
        let mut new_tasks = String::new();
        for task in tasks {
            if re.find(&task.data[0]).is_none() {
                new_tasks.push_str(&task.to_string());
            }
        }
        return self.write(new_tasks);
    }

    fn refresh(&mut self) -> Result<(), String> {
        let mut refresh_tasks = refresh::refresh();
        let tasks = self.parse()?;
        let mut new_tasks = Vec::new();
        for task in tasks {
            let mut found = false;
            for refresh_task in &refresh_tasks {
                if task.data[0] == refresh_task.data[0] {
                    found = true;
                }
            }
            if !found && !task.auto_delete {
                new_tasks.push(task);
            }
        }
        let mut ret = String::new();
        for task in new_tasks {
            for i in 0..refresh_tasks.len() {
                let refresh_task = &refresh_tasks[i];
                if task.data[3] < refresh_task.data[3] {
                    ret.push_str(&refresh_task.to_string());
                    refresh_tasks.remove(i);
                }
            }
            ret.push_str(&task.to_string());
        }
        for task in refresh_tasks {
            ret.push_str(&task.to_string());
        }
        return self.write(ret);
    }

    fn parse(&self) -> Result<Vec<Task>, String> {
        let file = handle(File::open(&self.path), "Failed to open tasks file.")?;
        let contents = io::BufReader::new(file).lines();
        let mut tasks = Vec::new();
        for line in contents {
            let line = handle(line, "Can't read from tasks file.")?;
            if line == "" {
                continue;
            }
            let mut task = Task {
                data: [String::new(), String::new(), String::new(), String::new()],
                auto_delete: false,
            };
            let mut i = 0;
            for field in line.split('|').take(4) {
                task.data[i] = field.to_string();
                i += 1;
            }
            if line.chars().last().ok_or("Empty line in tasks file.")? == '1' {
                task.auto_delete = true;
            }
            tasks.push(task);
        }
        return Ok(tasks);
    }

    fn write(&self, contents: String) -> Result<(), String> {
        let mut file = handle(File::create(&self.path), "Failed to open tasks file.")?;
        return handle(
            file.write_all(contents.as_bytes()),
            "Failed to write to tasks file.",
        );
    }
}

fn main() {
    use Command::*;
    let command = error(parse());
    let mut path = error(handle(current_exe(), "Failed to get executable path."));
    path.pop();
    path.push(".todo");
    let _ = create_dir(&path);
    path.push("tasks.txt");
    match File::open(&path) {
        Ok(_) => (),
        Err(_) => {
            error(handle(File::create(&path), "Failed to create tasks file."));
        }
    }
    let mut tasks = TaskFile::new(path);
    match command {
        Add(task) => error(tasks.add_task(task)),
        Remove(re) => error(tasks.remove_task(re)),
        List(re) => match re {
            Some(re) => {
                let tasks = error(tasks.parse());
                let re = error(handle(
                    Regex::new(&re),
                    "Invalid regular expression. See `todo --help` for usage.",
                ));
                for task in tasks {
                    if re.find(&task.data[0]).is_some() {
                        println!("* {}:", task.data[0]);
                        println!("  {}", task.data[1]);
                        println!("  Due by {}", task.data[2]);
                    }
                }
            }
            None => {
                let tasks = error(tasks.parse());
                for task in tasks {
                    println!("* {}:", task.data[0]);
                    println!("  {}", task.data[1]);
                    println!("  Due by {}", task.data[2]);
                }
            }
        },
        Refresh => error(tasks.refresh()),
        Clear => error(tasks.remove_task(".*".to_string())),
        Help => {
            println!("Usage: todo <command> [arg]");
            println!("Commands:");
            println!("  add <name> [description] [due date] [priority]");
            println!("  remove <regex>");
            println!("  list [regex]");
            println!("  refresh");
            println!("  clear");
            println!("  --help");
        }
    }
}

fn parse() -> Result<Command, String> {
    use Command::*;
    let mut args = std::env::args();
    let command = args
        .nth(1)
        .ok_or("Missing command. See `todo --help` for usage.")?;
    match command.as_str() {
        "add" => return Ok(Add(parse_task(args)?)),
        "remove" => {
            return Ok(Remove(
                args.nth(0)
                    .ok_or("Missing task to remove. See `todo --help` for usage.")?,
            ))
        }
        "list" => return Ok(List(args.nth(0))),
        "refresh" => return Ok(Refresh),
        "clear" => return Ok(Clear),
        "--help" => return Ok(Help),
        _ => return Err("Unrecognized command. See `todo --help` for usage.".to_string()),
    }
}

fn parse_task(mut args: Args) -> Result<Task, String> {
    let mut data = [
        String::new(),
        String::new(),
        String::new(),
        String::from("0"),
    ];
    let name = args
        .nth(0)
        .ok_or("Missing task to remove. See `todo --help` for usage.")?;
    data[0] = name;
    for i in 1..4 {
        match args.nth(0) {
            Some(field) => data[i] = field,
            None => break,
        }
    }
    return Ok(Task {
        data,
        auto_delete: false,
    });
}

fn handle<T, E>(result: Result<T, E>, message: &str) -> Result<T, String> {
    match result {
        Ok(ok) => return Ok(ok),
        Err(_) => return Err(message.to_string()),
    }
}

fn error<T>(result: Result<T, String>) -> T {
    match result {
        Ok(ok) => ok,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}

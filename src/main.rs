use regex::Regex;
use std::env::{current_exe, Args};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

enum Command {
    Add(Task),
    Remove(String),
    List(Option<String>),
    Refresh,
    Clear,
}

struct Task {
    data: [String; 4],
}

impl Task {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for field in &self.data {
            string.push_str(&field);
            string.push('|');
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

    fn add_task(&self, task: Task) {
        let tasks = self.parse();
        let mut new_tasks = String::new();
        let mut completed = false;
        let priority: u32 = task.data[3]
            .parse()
            .expect("Priority must be a number. See `todo --help` for usage.");
        for t in tasks {
            if !completed && t.data[3].parse::<u32>().unwrap() < priority {
                new_tasks.push_str(&task.to_string());
                completed = true;
            }
            new_tasks.push_str(&t.to_string());
        }
        if !completed {
            new_tasks.push_str(&task.to_string());
        }
        self.write(new_tasks);
    }

    fn remove_task(&mut self, re: String) {
        let tasks = self.parse();
        let re = Regex::new(&re).expect("Invalid regular expression.");
        let mut new_tasks = String::new();
        for t in tasks {
            if re.find(&t.data[0]).is_none() {
                new_tasks.push_str(&t.to_string());
            }
        }
        self.write(new_tasks);
    }

    fn parse(&self) -> Vec<Task> {
        let file = File::open(&self.path).expect("Failed to open tasks file.");
        let contents = io::BufReader::new(file).lines();
        let mut tasks = Vec::new();
        for line in contents {
            let line = line.unwrap();
            if line == "" {
                continue;
            }
            let mut task = Task {
                data: [String::new(), String::new(), String::new(), String::new()],
            };
            let mut i = 0;
            for field in line.split('|').take(4) {
                task.data[i] = field.to_string();
                i += 1;
            }
            tasks.push(task);
        }
        return tasks;
    }

    fn write(&self, contents: String) {
        let mut file = File::create(&self.path).expect("Failed to open tasks file.");
        file.write_all(contents.as_bytes())
            .expect("Failed to write to tasks file.");
    }
}

fn main() {
    use Command::*;
    let command = parse();
    let mut path = current_exe().unwrap();
    path.pop();
    path.push(".todo");
    path.push("tasks.txt");
    let mut tasks = TaskFile::new(path);
    match command {
        Add(task) => tasks.add_task(task),
        Remove(re) => tasks.remove_task(re),
        List(re) => match re {
            Some(re) => {
                let tasks = tasks.parse();
                let re = Regex::new(&re)
                    .expect("Invalid regular expression. See `todo --help` for usage.");
                for task in tasks {
                    if re.find(&task.data[0]).is_some() {
                        println!("{}", task.data[0]);
                    }
                }
                return;
            }
            None => {
                let tasks = tasks.parse();
                for task in tasks {
                    println!("{}", task.data[0]);
                }
                return;
            }
        },
        Refresh => {
            panic!("Not yet implemented");
        }
        Clear => tasks.remove_task(".*".to_string()),
    }
}

fn parse() -> Command {
    use Command::*;
    let mut args = std::env::args();
    let command = args
        .nth(1)
        .expect("Missing command. See `todo --help` for usage.");
    match command.as_str() {
        "add" => return Add(parse_task(args)),
        "remove" => {
            return Remove(
                args.nth(0)
                    .expect("Missing task to remove. See `todo --help` for usage."),
            )
        }
        "list" => return List(args.nth(0)),
        "refresh" => return Refresh,
        "clear" => return Clear,
        _ => panic!("Unrecognized command. See `todo --help` for usage."),
    }
}

fn parse_task(mut args: Args) -> Task {
    let mut data = [String::new(), String::new(), String::new(), String::from("0")];
    let name = args
        .nth(0)
        .expect("Missing task to remove. See `todo --help` for usage.");
    data[0] = name;
    for i in 1..4 {
        match args.nth(0) {
            Some(field) => data[i] = field,
            None => break,
        }
    }
    return Task { data };
}

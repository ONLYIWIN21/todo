use clap::{Args, Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use regex::Regex;
use std::env::current_exe;
use std::fs::{create_dir, File};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

mod refresh;

#[derive(Parser)]
#[command(name = "todo")]
struct Cli {
    #[clap(subcommand)]
    subcommand: Option<SubCommand>,
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,
}

#[derive(Subcommand)]
enum SubCommand {
    Add(Task),
    Remove(Remove),
    List(List),
    Refresh,
    Clear,
}

#[derive(Args)]
pub struct Task {
    name: String,
    description: String,
    due_date: String,
    priority: u32,
    auto_delete: bool,
}

#[derive(Args)]
struct Remove {
    re: String,
}

#[derive(Args)]
struct List {
    re: Option<String>,
}

impl Task {
    fn to_string(&self) -> String {
        let mut string = String::new();
        string.push_str(
            format!(
                "{}|{}|{}|{}|",
                self.name, self.description, self.due_date, self.priority
            )
            .as_str(),
        );
        if self.auto_delete {
            string.push('1');
        } else {
            string.push('0');
        }
        string.push('\n');
        return string;
    }

    fn from_string(string: String) -> Task {
        let mut task = Task {
            name: String::new(),
            description: String::new(),
            due_date: String::new(),
            priority: 0,
            auto_delete: false,
        };
        let mut data = string.split('|');
        task.name = option(data.next(), "Invalid tasks file.").to_string();
        task.description = option(data.next(), "Invalid tasks file.").to_string();
        task.due_date = option(data.next(), "Invalid tasks file.").to_string();
        task.priority = option(data.next(), "Invalid tasks file.").parse().unwrap();
        if option(data.next(), "Invalid tasks file.") == "1" {
            task.auto_delete = true;
        }
        return task;
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
        for t in tasks {
            if t.name == task.name {
                error(&format!("Duplicate task '{}'.", t.name));
            }
            if !completed && t.priority < task.priority {
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
        let re = result(
            Regex::new(&re),
            "Invalid regular expression. See `todo --help` for usage.",
        );
        let mut new_tasks = String::new();
        for task in tasks {
            if re.find(&task.name).is_none() {
                new_tasks.push_str(&task.to_string());
            }
        }
        self.write(new_tasks);
    }

    fn refresh(&mut self) {
        let mut refresh_tasks = refresh::refresh();
        let tasks = self.parse();
        let mut new_tasks = Vec::new();
        for task in tasks {
            if !task.auto_delete {
                new_tasks.push(task);
            }
        }
        let mut ret = String::new();
        for task in new_tasks {
            for i in 0..refresh_tasks.len() {
                let refresh_task = &refresh_tasks[i];
                if task.priority < refresh_task.priority {
                    ret.push_str(&refresh_task.to_string());
                    refresh_tasks[i].auto_delete = false;
                }
            }
            ret.push_str(&task.to_string());
        }
        for task in refresh_tasks {
            if task.auto_delete {
                ret.push_str(&task.to_string());
            }
        }
        self.write(ret);
    }

    fn parse(&self) -> Vec<Task> {
        let file = result(File::open(&self.path), "Failed to open tasks file.");
        let contents = io::BufReader::new(file).lines();
        let mut tasks = Vec::new();
        for line in contents {
            let line = result(line, "Can't read from tasks file.");
            if line == "" {
                continue;
            }
            let task = Task::from_string(line);
            tasks.push(task);
        }
        return tasks;
    }

    fn write(&self, contents: String) {
        let mut file = result(File::create(&self.path), "Failed to open tasks file.");
        result(
            file.write_all(contents.as_bytes()),
            "Failed to write to tasks file.",
        );
    }
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn main() {
    use SubCommand::*;
    let command = Cli::parse();
    if let Some(generator) = command.generator {
        let mut cmd = Cli::command();
        print_completions(generator, &mut cmd);
    }
    let mut path = result(current_exe(), "Failed to get executable path.");
    path.pop();
    path.push(".todo");
    let _ = create_dir(&path);
    path.push("tasks.txt");
    if File::open(&path).is_err() {
        result(File::create(&path), "Failed to create tasks file.");
    }
    if command.subcommand.is_none() {
        return;
    }
    let mut tasks = TaskFile::new(path);
    match command.subcommand.unwrap() {
        Add(task) => tasks.add_task(task),
        Remove(remove) => tasks.remove_task(remove.re),
        List(list) => match list.re {
            Some(re) => {
                let tasks = tasks.parse();
                let re = result(
                    Regex::new(&re),
                    "Invalid regular expression. See `todo --help` for usage.",
                );
                for task in tasks {
                    if re.find(&task.name).is_some() {
                        println!("* {}:", task.name);
                        println!("  {}", task.description);
                        println!("  Due by {}\n", task.due_date);
                    }
                }
            }
            None => {
                let tasks = tasks.parse();
                for task in tasks {
                    println!("* {}:", task.name);
                    println!("  {}", task.description);
                    println!("  Due by {}\n", task.due_date);
                }
            }
        },
        Refresh => tasks.refresh(),
        Clear => tasks.remove_task(".*".to_string()),
    }
}

fn option<T>(option: Option<T>, message: &str) -> T {
    match option {
        Some(some) => return some,
        None => error(message),
    }
}

fn result<T, E>(result: Result<T, E>, message: &str) -> T {
    match result {
        Ok(ok) => return ok,
        Err(_) => error(message),
    }
}

fn error(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

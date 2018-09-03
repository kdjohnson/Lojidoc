extern crate clap;
extern crate colored;
extern crate mdbook;
extern crate threadpool;

mod document;
mod grammar;
mod model;
mod parse;

use mdbook::config::Config;
use mdbook::MDBook;

use clap::App;
use clap::Arg;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use threadpool::ThreadPool;

use document::document::find_java_files;
use document::document::gen_md_book;
use document::document::generate_markdown;
use document::document::lint_javadoc;
use document::document::resolve_context;
use model::model::Project;
use parse::parse::parse_file;

/// Handles the single threaded option for running the application
///
/// # Arguments
///
/// * `file_paths` - A vector of the file paths of java files
/// * `dest` - The file path where the markdown will be saved
/// * `context` - The project context e.g. `github.com/user/repo`
/// * `verbose` - Whether the program will output verbose logging
pub fn document_single(
    file_paths: Vec<PathBuf>,
    dest: String,
    context: String,
    verbose: bool,
    book: bool,
) {
    let mut project: Project = Project::new();

    for file in file_paths.clone() {
        let mut class = parse_file(&file, verbose);

        let m_context = resolve_context(file, &context);

        if m_context != "" {
            class.ch_file_path(m_context);
        }
        if !class.is_class {
            project.add_interface(class.to_interface());
        } else {
            project.add_class(class.clone());
        }
    }

    generate_markdown(project, dest.as_str(), book);
    println!(
        "\nDocumentation finished. Generated {} markdown files.",
        file_paths.len()
    );
}

/// Handles the thread pooling the application
///
/// # Arguments
///
/// * `file_paths` - A vector of the file paths of java files
/// * `dest` - The file path where the markdown will be saved
pub fn document(
    file_paths: Vec<PathBuf>,
    dest: String,
    context: String,
    verbose: bool,
    book: bool,
) {
    let files = Arc::new(file_paths);
    let size = files.len();
    let mut pool_size = size / 4;

    if files.len() % 4 != 0 {
        pool_size += 1;
    }
    let pool = ThreadPool::new(pool_size);
    let safe_dest = Arc::new(dest);

    for i in 0..pool_size {
        let file_cp = files.clone();
        let new_dest = safe_dest.clone();
        let new_context = context.clone();

        pool.execute(move || {
            let mut project: Project = Project::new();

            for j in 0..4 {
                if (i * 4) + j < size {
                    let mut file = file_cp[(i * 4) + j].clone();
                    let mut class = parse_file(&file, verbose);
                    let m_context = resolve_context(file, &new_context);
                    class.ch_file_path(m_context);

                    if !class.is_class {
                        project.add_interface(class.to_interface());
                    } else {
                        project.add_class(class.clone());
                    }
                }
            }

            generate_markdown(project, new_dest.as_str(), book);
        });
    }

    pool.join();

    println!(
        "\nDocumentation finished. Generated {} markdown files.",
        files.len()
    );
}

fn main() {
    let matches = App::new("Javadoc-To-Markdown")
        .version("1.0.0")
        .author("Josh Brudnak <jobrud314@gmail.com>")
        .about("A tool for generating markdown documentation for java projects")
        .arg(
            Arg::with_name("INPUT")
                .value_name("FILE")
                .required(true)
                .help("Set the input directory to use")
                .index(1),
        ).arg(
            Arg::with_name("context")
                .help("Set the context path of the project")
                .value_name("FILE")
                .short("c"),
        ).arg(
            Arg::with_name("book")
                .value_name("FILE")
                .required(false)
                .short("b")
                .help("Use mdbook to create a book for your generated documentation"),
        ).arg(
            Arg::with_name("lint")
                .help("Check a java project for incorrent and missing javadocs")
                .short("l"),
        ).arg(
            Arg::with_name("verbose")
                .short("v")
                .help("Generate documentation for a project and provide verbose output"),
        ).arg(
            Arg::with_name("single-thread")
                .short("s")
                .help("Use only on thread for execution of the program"),
        ).arg(
            Arg::with_name("destination")
                .required(false)
                .value_name("FILE")
                .short("d")
                .help("Sets the destination directory of the created markdown files"),
        ).get_matches();

    let dir = matches
        .value_of("INPUT")
        .expect("Documentation directory not chosen")
        .to_string();
    let dest = matches
        .value_of("destination")
        .unwrap_or("./generated/")
        .to_string();

    let context = matches.value_of("context").unwrap_or("").to_string();
    let book = matches.value_of("book").unwrap_or("").to_string();
    let file_paths = find_java_files(Path::new(dir.clone().as_str()));
    let single_thread = matches.is_present("single_thread");
    let lint = matches.is_present("lint");
    let verbose = matches.is_present("verbose");

    fs::create_dir_all(dest.as_str()).expect("File path not able to be created");
    println!("\nGenerating documentation from {}\n", dir);

    if file_paths.len() > 0 {
        if book != "" {
            let mut cfg = Config::default();
            cfg.book.title = Some(book.clone());

            let init_res = MDBook::init("./markdown-book").with_config(cfg).build();

            if !init_res.is_ok() {
                println!("Error initializing markdown book");
            }
        }

        if single_thread && book != "" {
            document_single(file_paths, dest.clone(), context, verbose, true);
        } else if single_thread && book == "" {
            document_single(file_paths, dest.clone(), context, verbose, false);
        } else if lint && book != "" {
            lint_javadoc(file_paths, dest.clone(), true);
        } else if book != "" {
            document(file_paths, dest.clone(), context, verbose, true);
        } else {
            document(file_paths, dest.clone(), context, verbose, false);
        }

        if book != "" {
            gen_md_book(dest);
        }
    } else {
        println!("No java files found");
    }
}

use std::{collections::HashMap, time::Duration};
use chrono::prelude::*;

// How many components can be in a line. Needed in case of recursively defined components
const MAX_COMPONENT_DEPTH: u32 = 10;

#[derive(Clone)]
struct Post {
    name: String,
    file_name: String,
    content: String,
    date: Duration,
    date_hum: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("🔴 No arguments given. Please specify a path to the target directory. Aborting.");
        std::process::exit(0);
    }
    let verbose = args.contains(&"--verbose".to_string());
    let blog = args.contains(&"--blog".to_string());
    let path = &args[1];
    

    // Open Files
    println!("🟢 Building {}...", path);
    let paths = std::fs::read_dir(path.to_owned() + "\\text-src");
    let paths = match paths {
        Ok(paths) => paths,
        Err(_) => {
            println!("🔴 WARNING! ./{} does not exist. Aborting.", path);
            std::process::exit(0);
        }
    };

    // Page template
    let template = match std::fs::read_to_string(path.to_owned() + "/text-src/template.html") {
        Ok(string) => string,
        Err(_) => {
            println!("🔴 WARNING! Page template at ./{}/text-src/template.html does not exist. Aborting.", path);
            std::process::exit(0);
        }
    };


    // Only in --blog mode:
    let blog_paths = std::fs::read_dir(path.to_owned() + "\\text-src\\blog");
    let blog_paths = match blog_paths {
        Ok(blog_paths) => blog_paths,
        Err(_) => {
            println!("🔴 WARNING! ./{}/blog does not exist. In blog mode, this is were your posts/templates need to be saved. Aborting.", path);
            std::process::exit(0);
        }
    };

    // Blog template
    let post_template = if blog { match std::fs::read_to_string(path.to_owned() + "/text-src/blog/blog_post.html") {
        Ok(string) => string,
        Err(_) => {
            println!("🔴 WARNING! Blog post template at ./{}/text-src/blog/blog_post.html does not exist. Aborting.", path);
            std::process::exit(0);
        }
    }} else {
        String::new()
    };
    // Blog main page
    let index_template = if blog { match std::fs::read_to_string(path.to_owned() + "/text-src/blog/blog_index.html") {
        Ok(string) => string,
        Err(_) => {
            println!("🔴 WARNING! Blog index template at ./{}/text-src/blog/blog_index.html does not exist. Aborting.", path);
            std::process::exit(0);
        }
    }} else {
        String::new()
    };


    // Build components
    let components_string =
        std::fs::read_to_string(path.to_owned() + "/text-src/components.html").ok();

    // Html component hashmap
    let mut components: HashMap<String, String> = HashMap::new();
    if let Some(component_string) = components_string {
        parse_components(component_string, &mut components);
    } else {
        println!("🔴 WARNING! components.html is missing.")
    }

    // Populate html template components
    let template = populate_components(template, &components, verbose);
    let post_template = populate_components(post_template, &components, verbose);
    let index_template = populate_components(index_template, &components, verbose);

    // Build posts
    let mut count = 0;
    let mut posts: Vec<Post> = vec![];
    if blog {
        // Iterate over markdown files
        for md_path in blog_paths.filter(|x| {
            x.as_ref()
                .expect("path should exist")
                .file_name()
                .to_str()
                .expect("string should exist")
                .contains(".md")
        }) {
            match md_path {
                Ok(md_path) => {

                    // Get post name
                    let name = md_path.file_name();
                    let name_hum = name
                        .to_str()
                        .expect("should be valid unicode")
                        .split(".md")
                        .next()
                        .expect("should have .md");
                    if verbose {
                        println!("Markdown file: {}", name_hum);
                    }

                    // Read markdown
                    let file_content =
                        std::fs::read_to_string(md_path.path()).expect("file should exist");
                    if verbose {
                        println!("🟠 Markdown: \n{}", file_content);
                    }

                    // Populate components
                    let content_populated = populate_components(file_content, &components, verbose);

                    // Parse markdown with pulldown_cmark
                    let parse = pulldown_cmark::Parser::new(&content_populated);
                    let mut md_html = String::new();
                    pulldown_cmark::html::push_html(&mut md_html, parse);
                    if verbose {
                        println!("🟠 Generated HTML: \n{}", md_html);
                    }

                    // Save Post
                    let metadata = std::fs::metadata(md_path.path()).unwrap();

                    if let Ok(time) = metadata.created() {
                        posts.push(Post {
                            name: name_hum.to_string(),
                            file_name: name_hum.replace(" ", "-").to_lowercase() + ".html",
                            content: md_html.clone(),
                            date: time.duration_since(std::time::UNIX_EPOCH).unwrap(),
                            date_hum: Local.timestamp_opt(time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64, 0).unwrap().format("%d.%m.%Y - %H:%M").to_string(),
                        });
                    } else {
                        println!("🔴 WARNING! File creation date not supported on this platform!");
                        posts.push(Post {
                            name: name_hum.to_string(),
                            file_name: name_hum.replace(" ", "-").to_lowercase() + ".html",
                            content: md_html.clone(),
                            date: std::time::Duration::new(0, 0),
                            date_hum: Local.timestamp_opt(0, 0).unwrap().format("%d.%m.%Y - %H:%M").to_string(),
                        });
                    }

                    count += 1;
                }
                Err(_) => {}
            }
        }

        println!("🟢 Build {} post(s)", count);
        for post in &posts {
            println!("    - {}: {}", post.name, post.file_name)
        }

        // Generate blog index
        println!("🟢 Building index page...");
        posts.sort_by(|a, b| b.date.cmp(&a.date));

        let current_post_html = current_post_fmt(posts[0].clone());
        let all_posts_html = all_posts_table(&posts);

        let mut blog_index_html = index_template.replace("{{current_post}}", &current_post_html);
        blog_index_html = blog_index_html.replace("{{all_posts}}", &all_posts_html);

        // Save blog posts
        for post in posts {
            // Set title
            let mut html_file = post_template.replace("{{title}}", &post.name);
            // Insert formatted content
            html_file = html_file.replace("{{content}}", &post.content);
            // Insert date
            html_file = html_file.replace("{{date}}", &post.date_hum);
            // Insert all posts table
            html_file = html_file.replace("{{all_posts}}", &all_posts_html);
            // Insert current post
            html_file = html_file.replace("{{current_post}}", &current_post_html);

            // Write to disk. File names are lowercase and replace spaces with '-'
            std::fs::write(path.to_string() + "\\blog\\" + &post.file_name, html_file)
                .expect("should be able to write to file");
        }
        // Save blog index
        std::fs::write(path.to_string() + "\\blog\\blog.html", blog_index_html)
            .expect("should be able to write to file");

        println!("🟢 Build index page! Done.");
    }
    
}

fn current_post_fmt(post: Post) -> String {
    let dt = Local.timestamp_opt(post.date.as_secs() as i64, 0).unwrap().format("%d.%m.%Y - %H:%M").to_string();
    println!("🟢 Current Post: {} - {}", post.name, dt);

    let mut content = String::new();
    content += "<h1>Current: <a href=\"";
    content += &post.file_name;
    content += "\">";
    content += &post.name;
    content += "</a></h1>\n";
    content += &post.content;
    content += "\n<div class=\"blog_footer\">";
    content += &dt;
    content += "</div>";

    content
}

#[allow(unused)]
fn all_posts_list(posts: &Vec<Post>) -> String {
    let mut content = String::new();
    content += "<ul class=\"blog_post_list\">\n";

    for post in posts {
        content += "<li><a href=\"";
        content += &post.file_name;
        content += "\">";
        content += &post.name;
        content += "</a> - ";
        content += &Local.timestamp_opt(post.date.as_secs() as i64, 0).unwrap().format("%d.%m.%Y - %H:%M").to_string();
        content += "</li>\n";
    }
    content += "</ul>\n";

    content
}

fn all_posts_table(posts: &Vec<Post>) -> String {
    let mut content = String::new();
    content += "<table class=\"blog_post_list\">\n";

    for post in posts {
        content += "<tr><td><a href=\"";
        content += &post.file_name;
        content += "\">";
        content += &post.name;
        content += "</a></td><td> ";
        content += &Local.timestamp_opt(post.date.as_secs() as i64, 0).unwrap().format("%d.%m.%Y - %H:%M").to_string();
        content += "</td></tr>\n";
    }
    content += "</table>\n";

    content
}

fn parse_components(component_string: String, component_map: &mut HashMap<String, String>) {
    let mut comp_name = String::new();
    let mut comp = String::new();
    let mut is_comp = false;

    for line in component_string.lines() {
        if line.starts_with("{{") && line.chars().nth(2).expect("should be char") != '/' {
            is_comp = true;
        } else if line.starts_with("{{/") {
            is_comp = false;
        }

        if line.starts_with("{{") && is_comp {
            // Start of component
            comp_name = line.replace(['{', '}'], "");
        } else if line.starts_with("{{/") && !is_comp {
            // End of component, insert into map
            component_map.insert(comp_name.clone(), comp.clone());
            comp_name = String::new();
            comp = String::new();
        } else if is_comp {
            // Part of component
            comp = comp + line + "\n";
        }
    }
}

fn populate_components(content: String, components: &HashMap<String, String>, verbose: bool) -> String {
    let mut new_content = String::new();

    for line in content.lines() {
        if line.contains("{{component:") {
            if verbose {
                println!("🟠 Populating component(s): {}", line);
            }
            new_content = new_content + &comp_line(line, components, 0) + "\n"
        } else {
            new_content = new_content + line + "\n";
        }
    }

    new_content
}

// Recursively generate components
fn comp_line(line: &str, components: &HashMap<String, String>, depth: u32) -> String {
    let depth = depth + 1;
    if depth > MAX_COMPONENT_DEPTH {
        println!("🔴 WARNING! Maximum component depth reached. Is a component recursive?");
        return line.to_string();
    }
    if line.contains("{{component:") {
        let mut new_line = String::new();
        let split = line.split("{{component:").nth(1).expect("should be string");
        let name = split
            .split("}}")
            .nth(0)
            .expect("should be string")
            .replace(" ", "");

        let comp = components.get(&name);
        match comp {
            Some(comp) => {
                new_line = new_line
                    + &line.replace(
                        &("{{component:".to_string()
                            + split.split("}}").nth(0).expect("should be string")
                            + "}}"),
                        &comp,
                    );
                    new_line = comp_line(&new_line, components, depth);
                    return new_line;
            }
            None => {
                println!("🔴 WARNING! Component {} missing.", name);
                // Clear component
                new_line = new_line
                    + &line.replace(
                        &("{{component:".to_string()
                            + split.split("}}").nth(0).expect("should be string")
                            + "}}"),
                        "",
                    )
                    + "\n";
                new_line = comp_line(&new_line, components, depth);
                return new_line;
            }
        }
    } else {
        return line.to_string();
    }
}

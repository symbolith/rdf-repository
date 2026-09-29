use std::error::Error;
use std::fs::read_to_string;
use std::iter;
use std::path::{Path, PathBuf};

use hashlink::LinkedHashMap;
use markdown::mdast::{Heading, Link, List, ListItem, Node, Paragraph, Root, Text};
use markdown::{Constructs, ParseOptions};
use mdast_util_to_markdown::{IndentOptions, to_markdown_with_options};
use saphyr::{LoadableYamlNode, Scalar, Yaml};

struct Task<'t> {
    path: &'t Path,
    metadata: &'t LinkedHashMap<&'t str, Yaml<'t>>,
    task: &'t ListItem,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct LinkKey(Link);

impl std::hash::Hash for LinkKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.url.hash(state);
    }
}

const ALIASES: &str = "aliases";
const HEADING_NONE: &str = "None";

impl<'t> Task<'t> {
    fn get_link(&self) -> LinkKey {
        let url = self.path.to_string_lossy().to_string();
        LinkKey(Link {
            children: vec![Node::Text(Text {
                value: self
                    .get_link_title(ALIASES)
                    .map(String::from)
                    .unwrap_or(url.clone()),
                position: None,
            })],
            position: None,
            url,
            title: None,
        })
    }

    fn get_link_title(&self, title_key: &str) -> Option<&str> {
        match self.metadata.get(title_key)? {
            Yaml::Value(scalar) => scalar.as_str(),
            Yaml::Sequence(yamls) => yamls.first()?.as_str(),
            _ => None,
        }
    }
}

struct File<'input> {
    path: PathBuf,
    metadata: LinkedHashMap<&'input str, Yaml<'input>>,
    tasks: Vec<ListItem>,
}

use std::io::{self, BufRead};

fn get_tasks<'a>(file: &'a File<'a>) -> impl Iterator<Item = Task<'a>> {
    file.tasks.iter().map(|task| Task {
        path: &file.path,
        metadata: &file.metadata,
        task,
    })
}

fn group_tasks_by_metadata<'a>(
    tasks: impl Iterator<Item = Task<'a>>,
    key: &'a str,
) -> LinkedHashMap<Option<&'a str>, Vec<Task<'a>>> {
    let mut grouped: LinkedHashMap<Option<&str>, Vec<Task<'a>>> = LinkedHashMap::new();
    for task in tasks {
        let value = task.metadata.get(key).and_then(|yaml| match yaml {
            Yaml::Value(scalar) => scalar.as_str(),
            Yaml::Sequence(yamls) => yamls.first()?.as_str(),
            x => {
                let x = x.as_str();
                println!("{x:?}");
                None
            }
        });
        grouped.entry(value).or_insert_with(Vec::new).push(task);
    }
    grouped
}

fn _test() {
    let content = read_to_string("out/tasks.md").unwrap();

    let node = markdown::to_mdast(
        &content,
        &ParseOptions {
            constructs: Constructs {
                frontmatter: true,
                gfm_task_list_item: true,
                ..Constructs::default()
            },
            ..ParseOptions::default()
        },
    )
    .expect("parser does not throw");

    println!("{node:#?}");
}

fn group_tasks_by_link<'a>(
    tasks: impl Iterator<Item = Task<'a>>,
) -> LinkedHashMap<LinkKey, Vec<Task<'a>>> {
    let mut grouped: LinkedHashMap<LinkKey, Vec<Task<'a>>> = LinkedHashMap::new();

    for task in tasks {
        // println!("{0:?}", task.get_link());
        grouped
            .entry(task.get_link())
            .or_insert_with(Vec::new)
            .push(task);
    }
    grouped
}

fn main() -> Result<(), Box<dyn Error>> {
    let binding = std::env::args().skip(1).collect::<Vec<_>>();
    let mut hierarchy: Vec<&str> = binding.iter().map(|s| s.as_str()).collect();
    hierarchy.push("aliases");
    hierarchy.push("title");

    let mut files = vec![];

    for line in io::stdin().lock().lines() {
        let path = PathBuf::from(line?);

        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        let content = read_to_string(&path)?;

        let node = markdown::to_mdast(
            &content,
            &ParseOptions {
                constructs: Constructs {
                    frontmatter: true,
                    gfm_task_list_item: true,
                    ..Constructs::default()
                },
                ..ParseOptions::default()
            },
        )
        .expect("parser does not throw");

        let file = parse(node, path, &hierarchy)?;
        files.push(file);
    }

    let tasks = files.iter().flat_map(|file| get_tasks(file));
    let key = hierarchy.first().unwrap();

    let mut root_children = vec![];

    for (key, value) in group_tasks_by_metadata(tasks, key).into_iter() {
        root_children.push(Node::Heading(Heading {
            children: vec![Node::Text(Text {
                value: key.unwrap_or(HEADING_NONE).to_owned(),
                position: None,
            })],
            position: None,
            depth: 2,
        }));

        let mut children = vec![];
        for (LinkKey(link), tasks) in group_tasks_by_link(value.into_iter()) {
            let link_item = Node::ListItem(ListItem {
                children: vec![
                    Node::Paragraph(Paragraph {
                        children: vec![Node::Link(link)],
                        position: None,
                    }),
                    Node::List(List {
                        children: tasks
                            .into_iter()
                            .map(|task| Node::ListItem(task.task.clone()))
                            .collect(),
                        position: None,
                        ordered: false,
                        start: None,
                        spread: false,
                    }),
                ],
                position: None,
                spread: false,
                checked: None,
            });
            children.push(link_item);
        }
        root_children.push(Node::List(List {
            children,
            position: None,
            ordered: false,
            start: None,
            spread: true,
        }))
    }
    let root = Node::Root(Root {
        children: root_children,
        position: None,
    });
    let markdown = to_markdown_with_options(
        &root,
        &mdast_util_to_markdown::Options {
            bullet: '-',
            bullet_ordered: '.',
            bullet_other: '*',
            close_atx: false,
            emphasis: '*',
            fence: '`',
            fences: true,
            increment_list_marker: true,
            list_item_indent: IndentOptions::One,
            quote: '"',
            resource_link: false,
            rule: '*',
            rule_repetition: 3,
            rule_spaces: false,
            setext: false,
            single_dollar_text_math: true,
            strong: '*',
            tight_definitions: false,
        },
    )
    .unwrap();
    let mut cleaned = markdown.replace(r"\[", "[").replace(r"\(", "(");
    cleaned.insert_str(0, "---\naliases:\n  - Tasks\n---\n\n");
    println!("{cleaned}");
    Ok(())
}

fn parse<'node>(
    root: Node,
    path: PathBuf,
    hierarchy: &Vec<&'node str>,
) -> Result<File<'node>, Box<dyn Error>> {
    let metadatas = traverse_nodes(&root)
        .filter_map(|node| match node {
            Node::Yaml(yaml) => Some(yaml),
            _ => None,
        })
        .next()
        .and_then(|yaml| Yaml::load_from_str(&yaml.value).ok())
        .unwrap_or_default();

    let tasks = collect_unfinished_tasks(root);

    let mut metadata = LinkedHashMap::new();
    if let Some(first) = metadatas.first() {
        hierarchy.iter().for_each(|key| {
            if let Some(value) = get_value(key, first) {
                metadata.insert(*key, value);
            }
        });
    }
    Ok(File {
        path,
        metadata,
        tasks,
    })
}

fn traverse_nodes(root: &Node) -> impl Iterator<Item = &Node> {
    let mut stack = vec![root];
    iter::from_fn(move || {
        stack.pop().inspect(|node| {
            if let Some(children) = node.children() {
                stack.extend(children.iter().rev());
            }
        })
    })
}

fn get_value<'y>(key: &'_ str, yaml: &Yaml<'y>) -> Option<Yaml<'y>> {
    match yaml {
        Yaml::Mapping(linked_hash_map) => linked_hash_map
            .get(&Yaml::Value(Scalar::String(key.into())))
            .cloned(),
        _ => None,
    }
}

fn collect_unfinished_tasks(root: Node) -> Vec<ListItem> {
    let Node::Root(root) = root else {
        panic!("Expected Root node")
    };
    root.children
        .into_iter()
        .flat_map(prune_non_pending_list_items)
        .filter_map(|child| match child {
            Node::List(list) => Some(list.children),
            _ => None,
        })
        .flatten()
        .map(|child| match child {
            Node::ListItem(item) => item,
            _ => panic!("Expected ListItem node"),
        })
        .collect()
}

fn prune_non_pending_list_items(node: Node) -> Vec<Node> {
    match node {
        Node::ListItem(mut item) if item.checked == Some(false) => {
            item.children = item
                .children
                .into_iter()
                .flat_map(prune_non_pending_list_items)
                .collect();
            vec![Node::ListItem(item)]
        }
        Node::ListItem(item) => item
            .children
            .into_iter()
            .flat_map(prune_non_pending_list_items)
            .filter_map(|child| match child {
                Node::List(list) => Some(list.children),
                _ => None,
            })
            .flatten()
            .collect(),
        Node::List(mut list) => {
            list.children = list
                .children
                .into_iter()
                .flat_map(prune_non_pending_list_items)
                .collect();
            if list.children.is_empty() {
                vec![]
            } else {
                vec![Node::List(list)]
            }
        }
        node => vec![node],
    }
}

use std::fmt::{Display, Formatter};
use memmap::{Mmap, MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::ops::{Deref, DerefMut};

fn main() {
    // let manager = Manager::connect("./test")?;
    // manager.put("test", "test")
    let s: u8 = 0b1000_0000;
    println!("{}", s);
}

struct Page {
    file: MmapMut,
}

impl Page {
    const NODE_HEADER_INDEX: i32 = 0;
    const NODE_SORTED_TABLE_SIZE_INDEX: i32 = 1;
    const  NODE_SORTED_TABLE_INDEX: i32 = 2;

    fn new(file: &File, offset: u64, length: u64) -> std::io::Result<Page> {
        let mut mmap = unsafe {
            MmapOptions::new().offset(offset).len(length as usize).map_mut(file)?
        };
        let metadata = file.metadata()?;
        let file_length = metadata.len();
        if file_length == 0 {
            file.set_len(length)?;
        }
        Ok(Page { file: mmap })
    }

    fn get_header(&self) -> &u8 {
        self.file.get(Page::NODE_HEADER_INDEX as usize)
            .unwrap_or_else(|| panic!("failed reading NODE_HEADER"))
    }

    fn get_sorted_table_size(&self) -> &u8 {
        self.file.get(Page::NODE_SORTED_TABLE_SIZE_INDEX as usize)
            .unwrap_or_else(|| panic!("failed reading NODE_SORTED_TABLE_SIZE"))
    }
}

struct Node {
    page: Page,
}

impl Node {
    pub fn new(file: &File, offset: u64, length: u64) -> std::io::Result<Node> {
        let page = Page::new(file, offset, length)?;
        Ok(Node { page })
    }



    fn is_leaf(&self) -> bool {
        let node_header = self.page.get_header();
        node_header & 0b1000_0000 >> 7 == 0
    }

    pub fn put(&self, key: &str, value: &str) -> Result<()> {
        if self.is_leaf() {
            return Ok(());
        }


        Ok(())
    }

    fn search(&self, key: &str) -> Option<i32> {
        None
    }
}

struct Manager {
    file: File,
    root_node: Node,
}

const PAGE_SIZE: u64 = 64 * 1024;
const KEY_MAX_LENGTH: u32 = 255;

#[derive(Debug)]
struct Error {
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message.as_str())
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

impl Manager {
    fn connect(file_path: &str) -> std::io::Result<Manager> {
        let file = OpenOptions::new().read(true).write(true).create(true)
            .open(file_path)?;
        let root_node = Node::new(&file, 0, PAGE_SIZE)?;
        Ok(Manager { file, root_node })
    }

    fn put(&self, key: &str, value: &str) -> Result<()> {
        let key_bytes = key.bytes();
        if key_bytes.len() as u32 > KEY_MAX_LENGTH {
            return Err(Error { message: "key length exceed limit".to_string() });
        }
        self.root_node.put(key, value)?;
        Ok(())
    }
}
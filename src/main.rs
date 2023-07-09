use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::ops::DerefMut;

fn main() {
    // let manager = Manager::connect("./test")?;
    // manager.put("test", "test")
    let s: u8 = 0b1000_0000;
    println!("{}", s);
}

struct Page {
    mmap: MmapMut,
}

impl Page {
    const NODE_HEADER_POSITION: usize = 0;
    const NODE_HEADER_SIZE: usize = 1;
    const NODE_CURR_DATA_POINTER_POSITION: usize = Page::NODE_HEADER_SIZE;
    const NODE_CURR_DATA_POINTER_SIZE: usize = 4;
    const NODE_SORTED_TABLE_LENGTH_POSITION: usize = Page::NODE_CURR_DATA_POINTER_POSITION + Page::NODE_CURR_DATA_POINTER_SIZE;
    const NODE_SORTED_TABLE_LENGTH_SIZE: usize = 4;
    const NODE_SORTED_TABLE_POSITION: usize = Page::NODE_SORTED_TABLE_LENGTH_POSITION + Page::NODE_SORTED_TABLE_LENGTH_SIZE;

    fn new(file: &File, offset: u64, length: u64) -> std::io::Result<Page> {
        let mut mmap = unsafe {
            MmapOptions::new().offset(offset).len(length as usize).map_mut(file)?
        };
        let file_length = file.metadata()?.len();
        if file_length == 0 {
            file.set_len(length)?;
        }
        Ok(Page { mmap })
    }

    fn get_header(&self) -> u8 {
        *self.mmap.get(Page::NODE_HEADER_POSITION)
            .unwrap_or_else(|| panic!("failed reading NODE_HEADER"))
    }

    fn get_sorted_table_length(&self) -> u32 {
        let x = &self.mmap[Page::NODE_SORTED_TABLE_LENGTH_POSITION..Page::NODE_SORTED_TABLE_LENGTH_POSITION + Page::NODE_SORTED_TABLE_LENGTH_SIZE];
        u32::from_le_bytes(x.try_into().unwrap())
    }

    fn get_sorted_table(&self) -> Vec<usize> {
        let sorted_table_length = self.get_sorted_table_length() as usize;
        if sorted_table_length == 0 {
            return vec![];
        }
        let x = &self.mmap[Page::NODE_SORTED_TABLE_POSITION..Page::NODE_SORTED_TABLE_POSITION + sorted_table_length * 4];
        let mut sorted_table = Vec::with_capacity(sorted_table_length);
        for i in 0..sorted_table_length {
            sorted_table[i] = u32::from_le_bytes(x[i * 4..i * 4 + 4].try_into().unwrap()) as usize
        }
        sorted_table
    }

    fn search(&self, key: &[u8]) -> (bool, u32) {
        let sorted_table = self.get_sorted_table();
        if sorted_table.len() == 0 {
            return (false, 0);
        }
        let key_position = sorted_table.binary_search_by(|position| {
            let x = self.get_key(*position);
            x.cmp(key)
        });
        match key_position {
            Ok(p) => (true, p as u32),
            Err(e) => (false, e as u32)
        }
    }

    // 插入key value数据，返回是否成功，叶没有足够空间时会失败，这时叶需要分裂
    fn insert_at(&self, index: u32, key: &[u8], value: &[u8]) -> bool {
        todo!()
    }

    // 覆盖value数据，返回是否成功，叶没有足够空间时会失败，这时叶需要分裂
    fn override_value(&mut self, index: u32, value: &[u8]) -> bool {
        let (value_position_position, value_position) = self.get_value_position(index);
        let old_value = self.get_value_by_position(value_position);
        if old_value.cmp(value) == Ordering::Equal {
            return true;
        }
        let curr_data_pointer = self.get_curr_data_pointer();
        let new_curr_data_pointer = curr_data_pointer - Page::NODE_VALUE_SIZE_SIZE + value.len();
        let sorted_table_length = self.get_sorted_table_length();
        if new_curr_data_pointer < Page::NODE_SORTED_TABLE_POSITION + sorted_table_length as usize * 4 {
            return false;
        }
        let x = self.mmap.deref_mut();
        let binding = (value.len() as u32).to_le_bytes();
        let new_value_data: Vec<&u8> = binding.iter().chain(value.iter()).collect();
        // 写入新的value
        for i in new_curr_data_pointer..curr_data_pointer {
            x[i] = *new_value_data[i - new_curr_data_pointer];
        }
        let new_value_position_data = (new_curr_data_pointer as u32).to_le_bytes();
        // 更新value position
        for i in value_position_position..value_position_position + 4 {
            x[i] = new_value_position_data[i - value_position_position];
        }
        true
    }

    fn get_curr_data_pointer(&self) -> usize {
        let x = &self.mmap[Page::NODE_CURR_DATA_POINTER_POSITION..Page::NODE_CURR_DATA_POINTER_POSITION + Page::NODE_CURR_DATA_POINTER_SIZE];
        u32::from_le_bytes(x.try_into().unwrap()) as usize
    }

    fn get_value_by_position(&self, value_position: usize) -> &[u8] {
        let x = &self.mmap[value_position..value_position + Page::NODE_VALUE_SIZE_SIZE];
        let value_size = u32::from_le_bytes(x.try_into().unwrap());
        &self.mmap[value_position + Page::NODE_VALUE_SIZE_SIZE..value_position + Page::NODE_VALUE_SIZE_SIZE + value_size as usize]
    }

    fn get_value_by_index(&self, index: u32) -> &[u8] {
        todo!()
    }

    fn get_key_size(&self, position: usize) -> u32 {
        let x = &self.mmap[position..position + Page::NODE_KEY_SIZE_SIZE];
        u32::from_le_bytes(x.try_into().unwrap())
    }

    fn get_key(&self, position: usize) -> &[u8] {
        let key_size = self.get_key_size(position);
        let position = position + Page::NODE_KEY_SIZE_SIZE;
        &self.mmap[position..position + key_size as usize]
    }

    fn get_value_position(&self, index: u32) -> (usize, usize) {
        let sorted_table = self.get_sorted_table();
        let key_position = sorted_table.get(index as usize).unwrap();
        let key_size = self.get_key_size(*key_position);
        let value_position_position = key_position + Page::NODE_KEY_SIZE_SIZE + key_size as usize;
        let x = &self.mmap[value_position_position..value_position_position + Page::NODE_VALUE_POSITION_SIZE];
        let value_position = u32::from_le_bytes(x.try_into().unwrap());
        (value_position_position, value_position as usize)
    }

    const NODE_VALUE_SIZE_SIZE: usize = 4;
    const NODE_VALUE_POSITION_SIZE: usize = 4;
    const NODE_KEY_SIZE_SIZE: usize = 4;
}

struct Node {
    page: Page,
}

impl Node {
    const PAGE_SIZE: u64 = 64 * 1024;
    const KEY_MAX_LENGTH: u32 = 255;
    const VALUE_MAX_LENGTH: u32 = 512;

    pub fn new(file: &File, offset: u64, length: u64) -> std::io::Result<Node> {
        let page = Page::new(file, offset, length)?;
        Ok(Node { page })
    }

    pub fn put(&mut self, key: &str, value: &str) -> Result<Option<String>> {
        if self.is_leaf() {
            return self.put_leaf(key, value);
        }
        Ok(None)
    }

    // 向叶节点插入数据
    fn put_leaf(&mut self, key: &str, value: &str) -> Result<Option<String>> {
        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();
        let (exist, index) = self.page.search(key_bytes);
        return if !exist {
            let insert = self.page.insert_at(index, key_bytes, value_bytes);
            if !insert {
                panic!("failed putting data")
            }
            Ok(None)
        } else {
            let old_value = String::from_utf8(self.page.get_value_by_index(index).try_into().unwrap());
            if let Err(_) = old_value {
                return Err(Error { message: "failed decoding utf8".to_string() });
            }
            let insert = self.page.override_value(index, value_bytes);
            if !insert {
                panic!("failed putting data")
            }
            Ok(Some(old_value.unwrap()))
        };
    }

    // 判断是否为叶节点
    fn is_leaf(&self) -> bool {
        let node_header = self.page.get_header();
        node_header & 0b1000_0000 == 0
    }
}

struct Manager {
    file: File,
    root_node: Node,
}

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
    // 连接指定的索引文件
    fn connect(file_path: &str) -> std::io::Result<Manager> {
        let file = OpenOptions::new().read(true).write(true).create(true)
            .open(file_path)?;
        let root_node = Node::new(&file, 0, Node::PAGE_SIZE)?;
        Ok(Manager { file, root_node })
    }

    // 向索引插入数据，如果key已经有关联的value，覆盖并返回原value
    fn put(&mut self, key: &str, value: &str) -> Result<Option<String>> {
        if key.bytes().len() as u32 > Node::KEY_MAX_LENGTH {
            return Err(Error { message: "key length exceed limit".to_string() });
        }
        if value.bytes().len() as u32 > Node::VALUE_MAX_LENGTH {
            return Err(Error { message: "value length exceed limit".to_string() });
        }
        self.root_node.put(key, value)
    }
}
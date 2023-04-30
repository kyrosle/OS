use clap::{App, Arg};
use easyfs::{BlockDevice, EasyFileSystem};
use std::fs::{read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

const BLOCK_SZ: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
  fn read_block(&self, block_id: usize, buf: &mut [u8]) {
    let mut file = self.0.lock().unwrap();
    file
      .seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
      .expect("Error when seeking!");
    assert_eq!(
      file.read(buf).unwrap(),
      BLOCK_SZ,
      "Not a complete block!"
    );
  }

  fn write_block(&self, block_id: usize, buf: &[u8]) {
    let mut file = self.0.lock().unwrap();
    file
      .seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
      .expect("Error when seeking!");
    assert_eq!(
      file.write(buf).unwrap(),
      BLOCK_SZ,
      "Not a complete block!"
    );
  }
}

fn main() {
  easy_fs_pack().expect("Error when packing easy-fs!");
}

// #[derive(Parser)]
// #[clap(author, version, about, long_about = None)]
// struct App {
//   /// Executable source dir(with backslash)
//   #[clap(short, long)]
//   source: String,
//   /// Executable source dir(with backslash)
//   #[clap(short, long)]
//   target: String,
// }
fn easy_fs_pack() -> std::io::Result<()> {
  let matches = App::new("EasyFileSystem packer")
    .arg(
      Arg::with_name("source")
        .short("s")
        .long("source")
        .takes_value(true)
        .help("Executable source dir(with backslash)"),
    )
    .arg(
      Arg::with_name("target")
        .short("t")
        .long("target")
        .takes_value(true)
        .help("Executable target dir(with backslash)"),
    )
    .get_matches();
  let src_path = matches.value_of("source").unwrap();
  let target_path = matches.value_of("target").unwrap();
  println!(
    "src_path = {}\ntarget_path = {}\n",
    src_path, target_path
  );
  let block_file = Arc::new(BlockFile(Mutex::new({
    let f = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(format!("{}{}", target_path, "fs.img"))?;
    f.set_len(16 * 2048 * 512).unwrap();
    f
  })));

  // 4MiB, at most 4095 files
  let efs =
    EasyFileSystem::create(block_file, 16 * 2048, 1);

  let root_inode =
    Arc::new(EasyFileSystem::root_inode(&efs));
  let apps = read_dir(src_path)
    .unwrap()
    .map(|dir_entry| {
      let mut name_with_ext = dir_entry
        .unwrap()
        .file_name()
        .into_string()
        .unwrap();
      name_with_ext.drain(
        name_with_ext.find('.').unwrap()
          ..name_with_ext.len(),
      );
      name_with_ext
    })
    .collect::<Vec<_>>();

  println!("-- There are {} applications --", apps.len());

  for app in apps {
    // load app data from host file system
    println!("-- loading {} application to efs --", app);
    let mut host_file =
      File::open(format!("{}{}", target_path, app))
        .unwrap();
    let mut all_data: Vec<u8> = Vec::new();
    host_file.read_to_end(&mut all_data).unwrap();
    // create a file in easy-fs
    let inode = root_inode.create(app.as_str()).unwrap();
    // write data to easy-fs
    inode.write_at(0, all_data.as_slice());
  }
  // list apps
  // for app in root_inode.ls() {
  //   println!("{}", app);
  // }
  Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
  // efs block file
  let block_file = Arc::new(BlockFile(Mutex::new({
    let f = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open("target/fs.img")?;
    f.set_len(8192 * 512).unwrap();
    f
  })));
  // efs
  EasyFileSystem::create(block_file.clone(), 4096, 1);
  let efs = EasyFileSystem::open(block_file);
  let root_inode = EasyFileSystem::root_inode(&efs);

  // create `filea` and `fileb` inodes
  root_inode.create("filea");
  root_inode.create("fileb");
  for name in root_inode.ls() {
    println!("{}", name);
  }

  // acquire `filea` inode
  let filea = root_inode.find("filea").unwrap();

  // write `Hello, world!` to `filea`
  // read `Hello, world!` from `filea`
  let greet_str = "Hello, world!";
  filea.write_at(0, greet_str.as_bytes());
  // let mut buffer = [0u8; 512];
  let mut buffer = [0u8; 233];
  let len = filea.read_at(0, &mut buffer);
  assert_eq!(
    greet_str,
    core::str::from_utf8(&buffer[..len]).unwrap(),
  );

  let mut random_str_test = |len: usize| {
    // clean up the `filea` inode
    filea.clear();
    assert_eq!(
      filea.read_at(0, &mut buffer),
      0,
      "filea should be empty!"
    );

    let mut str = String::new();
    for _ in 0..len {
      // random digit
      str
        .push(char::from(b'0' + rand::random::<u8>() % 10));
    }

    filea.write_at(0, str.as_bytes());
    let mut read_buffer = [0u8; 127];
    let mut offset = 0usize;
    let mut read_str = String::new();
    loop {
      let len = filea.read_at(offset, &mut read_buffer);
      if len == 0 {
        break;
      }
      offset += len;
      read_str.push_str(
        core::str::from_utf8(&read_buffer[..len]).unwrap(),
      );
    }
    assert_eq!(str, read_str);
  };

  random_str_test(4 * BLOCK_SZ);
  random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
  random_str_test(100 * BLOCK_SZ);
  random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
  random_str_test((12 + 128) * BLOCK_SZ);
  random_str_test(400 * BLOCK_SZ);
  random_str_test(1000 * BLOCK_SZ);
  random_str_test(2000 * BLOCK_SZ);

  Ok(())
}

#[cfg(test)]

mod tests {
    use assert_cmd::{Command};
    use itertools::Itertools;
    use tempdir::{self, TempDir};
    use rust_embed::{self, Embed, RustEmbed};
    use std::path::{Path,PathBuf};
    use crate::misc_helper;
    
    //when true call it with 
    //  cargo test -- --test-threads 1 --nocapture
    const TESTS_DEBUG: bool = false;

    struct TestDirs {
        tmp_instance: TempDir,
        src: PathBuf,
        archive: PathBuf,
        dst: PathBuf,
    }

    impl TestDirs {
        fn new() -> TestDirs {
            let tmp_instance = TempDir::new("backup").unwrap();
            let tmp_dir = tmp_instance.path();

            println!("tmp: {:?}", tmp_dir);
    
            let src_dir = PathBuf::from(tmp_dir).join("src");
            let archive_dir = PathBuf::from(tmp_dir).join("archive");
            let dst_dir = PathBuf::from(tmp_dir).join("dst");

            misc_helper::create_dir_when_missing(&src_dir).expect("cannot create all test dirs");
            misc_helper::create_dir_when_missing(&dst_dir).expect("cannot create all test dirs");
            
            return TestDirs{
                tmp_instance: tmp_instance,
                src: src_dir,
                archive: archive_dir,
                dst: dst_dir,
            };
        }

        fn unpack<T: RustEmbed>(self) -> Self {
            for path_str in SimpleAsset::iter() {
                let path = PathBuf::from(path_str.as_ref()); //why &*?
    
                //create missing dirs
                if path.iter().count() > 1 {
                    let mut all_dirs = path.iter();
                    let _ = all_dirs.next_back();
                    let all_dirs = all_dirs.fold(PathBuf::from(&self.src), |mut s,part|{ s.join(part) });
                    misc_helper::create_dir_when_missing(&all_dirs).expect("cannot create all test dirs");
                }
    
                //create file
                let full_path = PathBuf::from(&self.src).join(path);
                let src_embed = T::get(&path_str).expect("cannot get test data");

                if TESTS_DEBUG {
                    println!("::: {:?}", full_path);
                }
      
                std::fs::write(full_path, src_embed.data);
            }

            return self;
        }

        fn archive_new(self) -> Self {
            let output = Command::cargo_bin("backup").unwrap()
            .arg(format!("--archive={}/", self.archive.to_string_lossy()))
            .arg("new")
            .unwrap();

            return self;
        }

        fn archive_backup(self) -> Self {
            let output = Command::cargo_bin("backup").unwrap()
            .arg(format!("--archive={}/", self.archive.to_string_lossy()))
            .arg("backup")
            .arg(format!("--source={}", self.src.to_string_lossy()))
            .arg(format!("--channel=main"))
            .unwrap();

            return self;
        }

        fn archive_restore(self) -> Self {
            let output = Command::cargo_bin("backup").unwrap()
            .arg(format!("--archive={}/", self.archive.to_string_lossy()))
            .arg("restore")
            .arg(format!("--destination={}", self.dst.to_string_lossy()))
            .arg(format!("--channel=main"))
            .unwrap();

            return self;
        }


    }

    //testdata
    #[derive(Embed)]
    #[folder = "examples/simple/"]
    struct SimpleAsset;

    #[test]
    fn simple() {
        let testdir = TestDirs::new()
            .unpack::<SimpleAsset>()
            .archive_new()
            .archive_backup()
            .archive_restore();

        //compare src with dst
        assert!(!dir_diff::is_different(&testdir.src, &testdir.dst).unwrap());

        //compare src with dst and create invalid dir
        let invalid_folder = PathBuf::from(&format!("{}/invalid_entry", &testdir.dst.to_string_lossy()));
        misc_helper::create_dir_when_missing(&invalid_folder).expect("cannot create all test dirs");
        assert!(dir_diff::is_different(&testdir.src, &testdir.dst).unwrap());

        if TESTS_DEBUG {
            println!("- {:?}", testdir.tmp_instance.into_path());
        }
    }
}



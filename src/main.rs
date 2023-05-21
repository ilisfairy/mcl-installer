mod aoe;

use self::aoe::AbortOnError;

use std::collections::HashMap;
use std::env;
use std::fmt::format;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::option::Option::Some;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use reqwest::{Client, Error, Response};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use indicatif::ProgressBar;
use tokio::task;
use tokio::sync::Mutex;
use zip::ZipArchive;

const MIRAI_REPO: &str = "mirai.mamoe.net/assets/mcl";

const PROG_VERSION: &str = "1.0.7";

fn get_os() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(any(target_os = "linux", target_os = "android"))]
    return "linux";
    #[cfg(target_os = "macos")]
    return "mac";
}

fn get_arch() -> &'static str {
    #[cfg(target_arch = "x86")]
    return "x32";
    #[cfg(target_arch = "x86_64")]
    return "x64";
    #[cfg(target_arch = "arm")]
    return "arm";
    #[cfg(target_arch = "aarch64")]
    return "aarch64";
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Package {
    announcement: Option<String>,
    #[serde(rename = "type")]
    package_type: Option<String>,
    channels: HashMap<String, Vec<String>>,
    repo: Option<HashMap<String, RepoInfo>>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct RepoInfo {
    archive: Option<String>,
    metadata: Option<String>,
}

#[inline]
fn str_to_int(s: &str) -> i32 {
    s.trim().parse::<i32>().expect("invalid number")
}

use lazy_static::lazy_static;
lazy_static! {
    static ref BUF: String = String::new();
}

fn read_line() -> String {
    io::stdout().flush().expect("flush failed");
    io::stdin().read_line(&mut BUF).expect("read_line failed");
    BUF.clone()
}

async fn get(client: &Mutex<Client>, str: &str) -> Result<Response, Error> {
    let client = client.lock().await.clone();
    tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(10), async {
            client.get(str)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.5672.127 Safari/537.36")
                .send()
                .await?
        })
        .await??
    })
    .await??
}
async fn unzip(path: &str) -> String {
    let result = task::spawn_blocking(move || {
        let mut zip = ZipExtract::new(path)?;
        zip.progress(|progress| {
            println!("Extracting {}%", progress);
        });
        zip.extract()?;
        let zip_file0 =  zip.zip _archive().by_index(0)?;
        zip_file0.name().to_owned()
    })
    .await??;

    result
}

async fn download(client: &Client, url: &str, file: &str) {
    println!("Start Downloading: {}", url);

    let res = client.head(url).send().await.aoe();
    let content_length = res
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .unwrap()
        .to_str()
        .aoe();
    let total = str_to_int(content_length);

    let pb = ProgressBar::new(total as u64);
    pb.set_style(indicatif::ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-"));

    let mut file = File::create(file).await.aoe();

    let mut current = 0;
    while current < total {
        let range_header = format!("bytes={}-{}", current, current + 10240 - 1);
        let range_value = HeaderValue::from_str(&range_header).aoe();
        let res = client.get(url).header(RANGE, range_value).send().await.aoe();

        let chunk = res.bytes().await.aoe();
        file.write_all(&*chunk).await.aoe();
        current += chunk.len();
        pb.set_position(current as u64);
    }

    pb.finish_with_message("Download completed");
}

fn get_canonical_path(p: &str) -> PathBuf {
    let p = PathBuf::from(p).canonicalize().aoe();
    #[cfg(windows)]
    return p.strip_prefix(r"\\?\").aoe();
    #[cfg(unix)]
    return p;
}

fn find_java() -> PathBuf {
    if PathBuf::from("./java").exists() {
        let j = get_canonical_path("java");
        #[cfg(target_os = "windows")]
        return j.join("bin/java.exe");
        #[cfg(any(target_os = "linux", target_os = "android"))]
        return j.join("bin/java");
        #[cfg(target_os = "macos")]
        return j.join("Contents/Home/bin/java");
    } else {
        if let Some(j) = env::var_os("JAVA_HOME") {
            let j = PathBuf::from(j);
            #[cfg(target_os = "windows")]
            return j.join("bin/java.exe");
            #[cfg(any(target_os = "linux", target_os = "android"))]
            return j.join("bin/java");
            #[cfg(target_os = "macos")]
            return j.join("Contents/Home/bin/java");
        } else {
            return PathBuf::from("java");
        }
    }
}

fn exec(cmd: &mut Command, msg: &str) -> Result<()> {
    let status = cmd.status().context(format!("while {}", msg))?;
    if !status.success() {
        anyhow::bail!("command exited with status {}", status);
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    self::aoe::register();

    let args = std::env::args().nth(1);
    let repo = if args.is_none() { MIRAI_REPO.to_string() } else { args.unwrap().to_string() };

    println!("iTXTech MCL Installer {} [OS: {}]", PROG_VERSION, get_os());
    println!("Licensed under GNU AGPLv3.");
    println!("https://github.com/iTXTech/mcl-installer");
    println!();
    println!(
        "iTXTech MCL and Java will be downloaded to \"{}\"",
        get_canonical_path(".")
    );
    println!();

    println!("Checking existing Java installation.");
    if !Path::new("./java").exists() {
        exec(
            Command::new("java").arg("-version"),
            "checking Java installation",
        );
    } else {
        exec(
            Command::new(find_java()).arg("-version"),
            "checking Java installation",
        );
        println!("Reinstall Java will delete the current installation.");
    };

    println!();
    print!("Would you like to install Java? (Y/N, default: Y) ");
    let install_java_opt = read_line().trim().to_lowercase();
    let install_java = install_java_opt.is_empty() || install_java_opt == "y";

    let client = reqwest::Client::new();

    let mut java = "java".to_string();
    if install_java {
        if Path::new("./java").exists() {
            println!("Deleting \"{}\".", get_canonical_path("java"));
            let _ = fs::remove_dir_all("java");
        }

        print!("Java version (11, 17, 18, 19), default: 18): ");
        let mut ver = str_to_int(&read_line());
        ver = if (11..=20).contains(&ver) { ver } else { 18 };

        print!("JRE or JDK (1: JRE, 2: JDK, default: JRE): ");
        let jre = if str_to_int(&read_line()) == 2 {
            "jdk"
        } else {
            "jre"
        };

        print!("Binary Architecture (default: {}): ", get_arch());
        let a = read_line();
        let arch = if a.trim().is_empty() {
            get_arch()
        } else {
            a.trim()
        };

        let url = format!(
            "https://mirrors.tuna.tsinghua.edu.cn/Adoptium/{}/{}/{}/{}/",
            ver,
            jre,
            arch,
            get_os()
        );
        println!("Fetching file list for {} version {} on {} from {}", jre, ver, arch, url);
        let resp = get(&client, &url)
            .await
            .aoe_msg("Fail to fetch AdoptOpenJDK download list");
        let text = resp.text().await.aoe();
        let pack = format!("OpenJDK{}U-{}", ver, jre);

        for line in text.split('\n') {
            if line.contains(&pack)
                && line.contains("hotspot")
                && (line.contains(".zip") || line.contains(".tar.gz"))
            {
                let start = line.find(&pack).unwrap();
                let end = line.find("\" title=\"").unwrap();
                let archive = format!("{}{}", url, &line[start..end]);

                download(&client, &archive, "java.arc").await;

                let java_dir;
                if cfg!(windows) {
                    java_dir = unzip("java.arc");
                } else {
                    let start = archive.find("hotspot_").unwrap();
                    let end = archive.find(".tar.gz").unwrap();
                    java_dir = format!(
                        "jdk-{}{}",
                        &archive[start + 8..end].replace("_", "+"),
                        if jre == "jre" { "-jre" } else { "" }
                    );
                }

                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    //tar.gz
                    let mut process = Command::new("tar")
                        .arg("-zxvf")
                        .arg("java.arc")
                        .stdout(Stdio::piped())
                        .spawn()
                        .aoe();
                    {
                        let lines = BufReader::new(process.stdout.as_mut().unwrap()).lines();
                        for line in lines {
                            print!("\rExtracting {}", line.aoe().trim().to_owned());
                        }
                    }
                    process.wait().aoe();
                    println!();
                }

                #[cfg(target_os = "macos")]
                {
                    println!("Extracting Archive...");
                    exec(
                        Command::new("tar").arg("-zxf").arg("java.arc"),
                        "decompressing Java",
                    );
                }

                fs::remove_file("java.arc").aoe();
                fs::rename(java_dir, "java").aoe();

                break;
            }
        }

        java = find_java();
        println!("Testing Java Executable: {}", java);
        Command::new(&java)
            .arg("-version")
            .spawn()
            .aoe()
            .wait()
            .aoe();
        println!();
    }

    if Path::new("mcl.jar").exists() {
        let mut zip = ZipArchive::new(File::open("mcl.jar").aoe()).aoe();
        let mut buf = String::new();
        zip.by_name("META-INF/MANIFEST.MF")
            .aoe()
            .read_to_string(&mut buf)
            .aoe();
        let start = buf.find("\nVersion: ").unwrap();
        let ver = &buf[start + 10..start + 23].to_string();
        let hyphen = ver.find('-').unwrap();
        let major = &ver[0..hyphen].to_string();
        let rev = &ver[hyphen + 1..ver.len()].to_string();

        println!("iTXTech Mirai Console Loader detected.");
        println!("Major Version: {} Revision: {}", major, rev);
        println!();
    }

    let manifest_url = format!("https://{}/org/itxtech/mcl/package.json", repo);
    println!("Fetching iTXTech MCL Package Info from {}", manifest_url);
    let manifest = get(&client, &manifest_url)
        .await
        .aoe()
        .json::<Package>()
        .await
        .aoe();
    println!("{}", manifest.announcement.unwrap());

    let latest = manifest
        .channels
        .get("stable")
        .unwrap()
        .last()
        .unwrap()
        .to_string();
    println!("The latest stable version of iTXTech MCL is {}", latest);

    print!("Would you like to download it? (Y/N, default: Y) ");
    let option = read_line().trim().to_lowercase();
    if option.is_empty() || option == "y" {
        let repo = manifest.repo.unwrap();
        let url = repo.get(&latest).unwrap().archive.as_ref().unwrap();
        download(&client, url, "mcl.zip").await;
        unzip("mcl.zip");
        let _ = fs::remove_file("mcl.zip");

        if install_java {
            #[cfg(windows)]
            if Path::new("mcl.cmd").exists() {
                let j = format!("set JAVA_BINARY=\"{}\"", java);
                fs::write(
                    "mcl.cmd",
                    fs::read_to_string("mcl.cmd")
                        .aoe()
                        .replace("set JAVA_BINARY=java", &j),
                );
            }

            #[cfg(unix)]
            if Path::new("mcl").exists() {
                let j = format!("export JAVA_BINARY=\"{}\"", java);
                let content = fs::read_to_string("mcl")
                    .aoe()
                    .replace("export JAVA_BINARY=java", &j);
                fs::write("mcl", content).aoe();
                exec(
                    Command::new("chmod").arg("777").arg("mcl"),
                    "setting permission to mcl",
                );
            }

            println!("MCL startup script has been updated.");
        }

        #[cfg(unix)]
        println!("Use \"./mcl\" to start MCL.");
        #[cfg(windows)]
        println!("Use \".\\mcl\" to start MCL.");

        println!();
    }

    println!("Press Enter to exit.");
    read_line();
}

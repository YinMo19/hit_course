use clap::Parser;
use colored::*;
use reqwest::Client;
use serde_json::Value;
use std::error::Error;
use std::fs::{self, File};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// cookie
    #[arg(short, long)]
    cookie: String,

    /// show curl command
    #[arg(short, long, default_value_t = false)]
    show: bool,
}

async fn curl_request(cookie: &str, id: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let response = client.post("http://jw.hitsz.edu.cn/Xsxk/addGouwuche")
        .header("Accept", "*/*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("Cookie", cookie)
        .header("Origin", "http://jw.hitsz.edu.cn")
        .header("Pragma", "no-cache")
        .header("Referer", "http://jw.hitsz.edu.cn/Xsxk/query/1")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0")
        .header("X-Requested-With", "XMLHttpRequest")
        .body(format!("cxsfmt=0&p_pylx=1&mxpylx=1&p_sfgldjr=0&p_sfredis=0&p_sfsyxkgwc=0&p_xktjz=rwtjzyx&p_chaxunxh=&p_gjz=&p_skjs=&p_xn=2024-2025&p_xq=2&p_xnxq=2024-20252&p_dqxn=2024-2025&p_dqxq=1&p_dqxnxq=2024-20251&p_xkfsdm=xx-b-b&p_xiaoqu=&p_kkyx=&p_kclb=&p_xkxs=&p_dyc=&p_kkxnxq=&p_id={}&p_sfhlctkc=0&p_sfhllrlkc=0&p_kxsj_xqj=&p_kxsj_ksjc=&p_kxsj_jsjc=&p_kcdm_js=&p_kcdm_cxrw=&p_kc_gjz=&p_xzcxtjz_nj=&p_xzcxtjz_yx=&p_xzcxtjz_zy=&p_xzcxtjz_zyfx=&p_xzcxtjz_bj=&p_sfxsgwckb=1&p_skyy=&p_chaxunxkfsdm=&pageNum=1&pageSize=18", id))
        .send()
        .await?
        .text()
        .await?;

    Ok(response)
}

#[allow(unused)]
fn curl_request_str(cookie: &str, id: ColoredString) -> String {
    format!(
        r#"curl "http://jw.hitsz.edu.cn/Xsxk/addGouwuche" \
    -H 'Accept: */*' \
    -H 'Accept-Language: zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6' \
    -H 'Cache-Control: no-cache' \
    -H 'Connection: keep-alive' \
    -H 'Content-Type: application/x-www-form-urlencoded; charset=UTF-8' \
    -H 'Cookie: {}' \
    -H 'Origin: http://jw.hitsz.edu.cn' \
    -H 'Pragma: no-cache' \
    -H 'Referer: http://jw.hitsz.edu.cn/Xsxk/query/1' \
    -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0' \
    -H 'X-Requested-With: XMLHttpRequest' \
    --data-raw 'cxsfmt=0&p_pylx=1&mxpylx=1&p_sfgldjr=0&p_sfredis=0&p_sfsyxkgwc=0&p_xktjz=rwtjzyx&p_chaxunxh=&p_gjz=&p_skjs=&p_xn=2024-2025&p_xq=2&p_xnxq=2024-20252&p_dqxn=2024-2025&p_dqxq=1&p_dqxnxq=2024-20251&p_xkfsdm=xx-b-b&p_xiaoqu=&p_kkyx=&p_kclb=&p_xkxs=&p_dyc=&p_kkxnxq=&p_id={}&p_sfhlctkc=0&p_sfhllrlkc=0&p_kxsj_xqj=&p_kxsj_ksjc=&p_kxsj_jsjc=&p_kcdm_js=&p_kcdm_cxrw=&p_kc_gjz=&p_xzcxtjz_nj=&p_xzcxtjz_yx=&p_xzcxtjz_zy=&p_xzcxtjz_zyfx=&p_xzcxtjz_bj=&p_sfxsgwckb=1&p_skyy=&p_chaxunxkfsdm=&pageNum=1&pageSize=18' \
    --insecure"#,
        cookie, id
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let courses_dir = Path::new("./all_courses");

    if !courses_dir.is_dir() {
        eprintln!("The directory {} does not exist.", courses_dir.display());
    }

    for entry in fs::read_dir(courses_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file = File::open(&path).expect("unable to open file.");
            let must_choose: Value = serde_json::from_reader(file)?;
            let cookie = &args.cookie;
            let show_curl = args.show;

            let _res = choose_course(
                must_choose["kxrwList"]["list"]
                    .as_array()
                    .unwrap_or(&vec![]),
                cookie,
                show_curl,
            )
            .await?;
        }
    }

    Ok(())
}

async fn choose_course(
    all_course: &Vec<Value>,
    cookie: &str,
    show_curl: bool,
) -> Result<(), Box<dyn Error>> {
    for course in all_course {
        println!(
            "\n\nCourse id: {}, Course name: {}, Course teacher: {} {}",
            course["id"].as_str().unwrap().green().bold(),
            course["kcmc"].as_str().unwrap().purple().bold(),
            course["dgjsmc"].as_str().unwrap_or("").blue().bold(),
            course["tyxmmc"].as_str().unwrap_or("").blue().bold(),
        );

        println!(
            "{}",
            "Choose this? Yes: Y/y/YES/Yes/YeS/yeS/yEs/yeS/yes, else no."
                .green()
                .bold()
        );
        let mut buffer = String::new();
        let stdin = std::io::stdin(); // We get `Stdin` here
        stdin.read_line(&mut buffer)?;
        let yes = buffer.trim().to_lowercase() == "y" || buffer.trim().to_lowercase() == "yes";

        if yes {
            if show_curl {
                println!(
                    "{}\n{}\n",
                    "request curl command:".green().bold(),
                    curl_request_str(cookie, course["id"].as_str().unwrap().green().bold())
                );
            }

            let res = curl_request(cookie, course["id"].as_str().unwrap()).await?;
            println!("Response: {}", res);
        } else {
            println!("{}", "Skip this course.".red().bold());
            continue;
        }
    }

    Ok(())
}

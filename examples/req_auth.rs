use clap::Parser;
use colored::*;
use reqwest::redirect::Policy;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 用户名
    #[arg(short, long)]
    username: String,

    /// 密码
    #[arg(short, long)]
    password: String,

    /// 默认从文件夹中读取 JSON（的课程信息）。
    /// 如果加上此参数'./req_auth -s' 则会从当前目录读 `pre_select.json`进行直接选课。
    #[arg(short, long, default_value_t = false)]
    selected_json: bool,

    /// 存放json 的文件夹地址，默认 all_courses
    #[arg(short, long)]
    folder_addr: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // parse params
    let args = Args::parse();

    let path = args.folder_addr.unwrap_or("./all_courses".to_string());
    let courses_dir = Path::new(&path);

    if !courses_dir.is_dir() {
        eprintln!("The directory {} does not exist.", courses_dir.display());
    }

    // new client, with cookie store
    let client = Client::builder()
        .redirect(Policy::limited(5))
        .cookie_store(true)
        .build()
        .unwrap();

    // authenticate
    println!("{}", "Authenticating...".blue().bold());
    let response = client
        .get("https://ids.hit.edu.cn/authserver/combinedLogin.do?type=IDSUnion&appId=ff2dfca3a2a2448e9026a8c6e38fa52b&success=http%3A%2F%2Fjw.hitsz.edu.cn%2FcasLogin")
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(&response);
    let input_selector = Selector::parse("form#authZForm input[type=hidden]").unwrap();

    let mut form_data = HashMap::new();
    form_data.insert("username", args.username);
    form_data.insert("password", args.password);

    for input in document.select(&input_selector) {
        if let Some(name) = input.value().attr("name") {
            if let Some(value) = input.value().attr("value") {
                form_data.insert(name, value.to_string());
            }
        }
    }

    let response = client
        .post("https://sso.hitsz.edu.cn:7002/cas/oauth2.0/authorize")
        .form(&form_data)
        .send()
        .await?;

    if response.status().is_success() {
        println!("{}", "Login success.".green().bold());
    } else {
        println!("{}", "Login failed.".red().bold());
        return Ok(());
    }
    // auth end.

    let mut pre_select: Value = Value::Null;

    if !args.selected_json {
        for entry in fs::read_dir(courses_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let file = File::open(&path).expect("unable to open file.");
                let to_choose: Value = serde_json::from_reader(file)?;

                pre_select = to_choose.clone();
                pre_select["kxrwList"]["list"]
                    .as_array_mut()
                    .unwrap()
                    .clear();

                list_all_course(to_choose["kxrwList"]["list"].as_array().unwrap_or(&vec![]));
            }
        }
    } else {
        let file = File::open("./pre_select.json").expect("unable to open file.");
        let to_choose: Value = serde_json::from_reader(file)?;
        list_all_course(to_choose["kxrwList"]["list"].as_array().unwrap_or(&vec![]));

        let _res = choose_course(
            to_choose["kxrwList"]["list"].as_array().unwrap_or(&vec![]),
            client.clone(),
            &to_choose,
            &mut pre_select,
            true,
        )
        .await?;
        return Ok(());
    }

    loop {
        for entry in fs::read_dir(courses_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let file = File::open(&path).expect("unable to open file.");
                let to_choose: Value = serde_json::from_reader(file)?;
                let _res = choose_course(
                    to_choose["kxrwList"]["list"].as_array().unwrap_or(&vec![]),
                    client.clone(),
                    &to_choose,
                    &mut pre_select,
                    false,
                )
                .await?;
            }
        }

        // save pre_select to pre_select.json
        let pre_select_file = File::create("pre_select.json")?;
        serde_json::to_writer(pre_select_file, &pre_select)?;
        println!("{}", "pre_select.json saved.".purple().bold());
        println!("{}", "A NEW TERM BEGINS.\n----\n----\n----".purple().bold());
    }
}

async fn choose_course(
    all_course: &Vec<Value>,
    client: Client,
    to_choose: &Value, // for curl request param: xn/xq/...
    pre_select: &mut Value,
    use_pre_select: bool,
) -> Result<(), Box<dyn Error>> {
    for course in all_course {
        println!(
            "\n\nCourse id: {}, Course name: {}, Course teacher: {} {}",
            course["id"].as_str().unwrap().green().bold(),
            course["kcmc"].as_str().unwrap().purple().bold(),
            course["dgjsmc"].as_str().unwrap_or("").blue().bold(),
            course["tyxmmc"].as_str().unwrap_or("").blue().bold(),
        );

        let (yes, select) = if !use_pre_select {
            println!(
                "{} {}",
                "Choose this? Yes: Y/y/yes, else no.".green().bold(),
                "Input Select/S/s to add to pre_select.json".blue().bold(),
            );
            let mut buffer = String::new();
            let stdin = std::io::stdin(); // We get `Stdin` here
            stdin.read_line(&mut buffer)?;
            let buffer = buffer.trim().to_lowercase();
            let yes = buffer == "y" || buffer == "yes";
            let select = buffer == "s" || buffer == "select";
            (yes, select)
        } else {
            (true, false)
        };

        if yes {
            let max_retries = 3;
            let mut attempt = 0;

            while attempt < max_retries {
                let res =
                    curl_request(client.clone(), course["id"].as_str().unwrap(), to_choose).await;
                match res {
                    Ok(res) => {
                        println!("{} {}", "Success:".green().bold(), res);
                        break;
                    }
                    Err(e) => {
                        attempt += 1;
                        if attempt < max_retries {
                            println!(
                                "{} {} (Attempt {}/{})",
                                "Error:".red().bold(),
                                e.to_string(),
                                attempt,
                                max_retries
                            );
                            sleep(Duration::from_secs(3));
                        } else {
                            println!(
                                "{} {} (Max retries reached)",
                                "Error:".red().bold(),
                                e.to_string()
                            );
                        }
                    }
                }
            }

            if use_pre_select {
                sleep(Duration::from_secs(2));
            }
        } else if select {
            pre_select["kxrwList"]["list"]
                .as_array_mut()
                .unwrap()
                .push(course.clone());

            println!("{}", "selected.".blue().bold());
            continue;
        } else {
            println!("{}", "Skip this course.".red().bold());
            continue;
        }
    }

    Ok(())
}

fn list_all_course(all_course: &Vec<Value>) {
    for course in all_course {
        println!(
            "Course id: {}, Course name: {}, Course teacher: {} {}",
            course["id"].as_str().unwrap().green().bold(),
            course["kcmc"].as_str().unwrap().purple().bold(),
            course["dgjsmc"].as_str().unwrap_or("").blue().bold(),
            course["tyxmmc"].as_str().unwrap_or("")
        )
    }
}

async fn curl_request(
    client: Client,
    id: &str,
    to_choose: &Value,
) -> Result<String, Box<dyn Error>> {
    let response = client.post("http://jw.hitsz.edu.cn/Xsxk/addGouwuche")
        .header("Connection", "keep-alive")
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .body(format!("cxsfmt=0&p_pylx={}&mxpylx=1&p_sfgldjr={}&p_sfredis=0&p_sfsyxkgwc=0&p_xktjz=rwtjzyx&p_chaxunxh=&p_gjz=&p_skjs=&p_xn={}&p_xq={}&p_xnxq={}&p_dqxn={}&p_dqxq={}&p_dqxnxq={}&p_xkfsdm={}&p_xiaoqu=&p_kkyx=&p_kclb=&p_xkxs=&p_dyc=&p_kkxnxq=&p_id={}&p_sfhlctkc=0&p_sfhllrlkc=0&p_kxsj_xqj=&p_kxsj_ksjc=&p_kxsj_jsjc=&p_kcdm_js=&p_kcdm_cxrw=&p_kc_gjz=&p_xzcxtjz_nj=&p_xzcxtjz_yx=&p_xzcxtjz_zy=&p_xzcxtjz_zyfx=&p_xzcxtjz_bj=&p_sfxsgwckb=1&p_skyy=&p_chaxunxkfsdm=&pageNum=1&pageSize=18",
            to_choose["xsxkPage"]["p_pylx"]
                .as_str()
                .expect("no semister specified."),
            to_choose["xsxkPage"]["p_sfgldjr"]
                .as_str()
                .expect("no semister specified."),
            to_choose["xsxkPage"]["p_xn"]
                .as_str()
                .expect("no year specified."),
            to_choose["xsxkPage"]["p_xq"]
                .as_str()
                .expect("no semister specified."),
            to_choose["xsxkPage"]["p_xnxq"]
                .as_str()
                .expect("no year_semister specified."),
            to_choose["xsxkPage"]["p_dqxn"]
                .as_str()
                .expect("no semister specified."),
            to_choose["xsxkPage"]["p_dqxq"]
                .as_str()
                .expect("no semister specified."),
            to_choose["xsxkPage"]["p_dqxnxq"]
                .as_str()
                .expect("no ...dqxnxq specified."),
            to_choose["xsxkPage"]["p_xkfsdm"]
                .as_str()
                .expect("no method specified."),
            // "xx-b-b",
            id))
        .send()
        .await?
        .text()
        .await?;

    Ok(response)
}

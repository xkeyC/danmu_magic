use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use xml::EventReader;
use xml::reader::XmlEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct DanMuModel {
    // 弹幕发送时间
    pub send_time: f64,
    // 弹幕内容
    pub content: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpApiResp {
    pub code: i64,
    pub message: String,
    pub ttl: i64,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析弹幕文件
    let xml_reader = BufReader::new(File::open("./danmaku.xml")?); // Buffering is important for performance
    let parser = EventReader::new(xml_reader);
    let mut dan_mu_list = Vec::new();
    let mut working_dan_mu = DanMuModel {
        send_time: 0.0,
        content: "".to_string(),
    };
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, attributes, namespace: _namespace }) => {
                working_dan_mu = DanMuModel {
                    send_time: 0.0,
                    content: "".to_string(),
                };
                if name.local_name == "d" {
                    attributes.iter().for_each(
                        |attr| {
                            if attr.name.local_name == "p" {
                                let p = attr.value.split(",").collect::<Vec<&str>>();
                                working_dan_mu.send_time = p[0].parse::<f64>().unwrap();
                            }
                        }
                    )
                }
                if name.local_name == "sc" {
                    attributes.iter().for_each(
                        |attr| {
                            if attr.name.local_name == "ts" {
                                working_dan_mu.send_time = attr.value.parse::<f64>().unwrap();
                            }
                        }
                    )
                }
            }
            Ok(XmlEvent::Characters(s)) => {
                working_dan_mu.content = s;
            }

            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "d" || name.local_name == "sc" {
                    dan_mu_list.push(working_dan_mu.clone());
                    // println!("解析弹幕：{:?}", working_dan_mu);
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            _ => {}
        }
    }
    println!("已装载弹幕数量 -> {:?}", dan_mu_list.len());
    // 读取 cookies.txt
    let cookies = std::fs::read_to_string("./cookies.txt")?;
    println!("请输入视频CID/OID（按回车键结束）：");
    let mut cid = String::new();
    std::io::stdin().read_line(&mut cid)?;
    println!("请输入视频BV号（按回车键结束）：");
    let mut bv = String::new();
    std::io::stdin().read_line(&mut bv)?;
    println!("请输入偏移时长(单位：秒)（按回车键结束）：");
    let mut offset = String::new();
    std::io::stdin().read_line(&mut offset)?;
    let offset = offset.trim().parse::<f64>().unwrap();
    println!("请输入 csrf（按回车键结束）：");
    let mut csrf = String::new();
    std::io::stdin().read_line(&mut csrf)?;

    for (index, d) in dan_mu_list.iter().enumerate() {
        println!("正在发送弹幕：{:?} .... ({:?}/{:?})", d.content, index, dan_mu_list.len());
        send_dan_mu(d, cid.clone(), offset.clone(), csrf.clone(), cookies.clone(), bv.clone()).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
    Ok(())
}

async fn send_dan_mu(dan_mu: &DanMuModel, cid: String, offset: f64, csrf: String, cookies: String, bv: String) -> anyhow::Result<()> {
    // https://api.bilibili.com/x/v2/dm/post
    // POST
    // Content-Type: application/x-www-form-urlencoded

    let progress = ((dan_mu.send_time * 1000.0 + offset * 1000.0) as i64).to_string();
    let url = "https://api.bilibili.com/x/v2/dm/post";
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("type", "1");
    params.insert("oid", cid.trim());
    params.insert("bvid", bv.trim());
    params.insert("msg", dan_mu.content.as_str());
    params.insert("progress", progress.as_str());
    params.insert("pool", "0");
    params.insert("mode", "1");
    params.insert("rnd", "2");
    params.insert("csrf", csrf.trim());

    loop {
        let resp =
            client.post(url)
                .header("Accept", "application/json, text/plain")
                .header("Origin", "https://www.bilibili.com")
                .header("Referer", "https://www.bilibili.com/")
                .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
                .header("Cookie", cookies.as_str())
                .form(&params)
                .send().await?;
        let status = resp.status();
        let data = resp.json::<HttpApiResp>().await?;
        if data.code == 0 {
            println!("发送成功：{:?} ({:?})", data, status);
            break;
        } else {
            println!("发送失败：{:?} ({:?})，一分钟后重试", data, status);
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
    Ok(())
}

use std::fs;

use anyhow::Result;
use thirtyfour::WebDriver;

pub fn setup() {
    // docker-compose up -d
    // docker-compose down
    let _ = env_logger::builder().is_test(true).try_init();
}

pub async fn get_file_content(driver: &WebDriver, uri: &str) -> Result<String> {
    let script = r#"
  var callback = arguments[0];

  var xhr = new XMLHttpRequest();
  xhr.responseType = 'text/plain';
  xhr.onload = function() {
    var paragraph = document.createElement("pre");
    paragraph.innerText = xhr.response;
    document.body.appendChild(paragraph);
    callback(paragraph)
  };
  xhr.onerror = function(){ callback(xhr.status) };
  xhr.open('GET', '{uri}');
  xhr.send();
  "#;

    let args = vec![serde_json::to_value(uri)?];
    log::debug!("Execute remote script: ->{}<-", &script);

    let result = driver.execute_async(script, args).await?;
    let element = result.element()?;
    let text = element.text().await?;
    log::debug!("text got from script: ->{}<-", &text);
    Ok(text)
}

pub fn load_test_file(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn compare_strs_by_line(file1: &str, file2: &str) {
    let mut line_number = 1;
    let mut iter1 = file1.lines();
    let mut iter2 = file2.lines();
    while let (Some(line1), Some(line2)) = (iter1.next(), iter2.next()) {
        assert_eq!(
            line1, line2,
            "comparing lines in files, line number: {}",
            line_number
        );
        line_number += 1;
    }
}

mod common;

use thirtyfour::{http::reqwest_async::ReqwestDriverAsync, prelude::*, GenericWebDriver};

async fn fill_all_fields(driver: &GenericWebDriver<ReqwestDriverAsync>) -> WebDriverResult<String> {
    driver.get("http://localhost:8080").await?;

    let name_input = driver.find_element(By::Id("name")).await?;
    name_input.send_keys("Niles").await?;

    let surname_input = driver.find_element(By::Id("surname")).await?;
    surname_input.send_keys("Smith Doncic").await?;

    let nif_input = driver.find_element(By::Id("nif")).await?;
    nif_input.send_keys("12345689A").await?;

    let year_input = driver.find_element(By::Id("year")).await?;
    year_input.send_keys("2019").await?;

    let degiro_path_string = String::from("/test_data/degiro_2019.pdf");
    log::info!("degiro report path: ->{}<-", degiro_path_string);

    let degiro_report = driver.find_element(By::Id("degiro_report")).await?;
    degiro_report
        .send_keys(TypingData::from(degiro_path_string))
        .await?;

    let ib_path_string = String::from("/test_data/Annuals.2019.zip");
    log::info!("ib report path: ->{}<-", ib_path_string);

    let ib_report = driver.find_element(By::Id("ib_report")).await?;
    ib_report
        .send_keys(TypingData::from(ib_path_string))
        .await?;

    // thread::sleep(time::Duration::from_secs(1));

    let aeat_720_form = driver.find_element(By::Id("aeat_720_form")).await?;
    let href_aeat_720_form = aeat_720_form
        .get_attribute("href")
        .await?
        .expect("href for form should have been generated");

    log::info!("href aeat 720 form: ->{}<-", href_aeat_720_form);
    let aeat_720_form = common::get_file_content(driver, &href_aeat_720_form)
        .await
        .expect("blob string");

    Ok(aeat_720_form)
}

fn check_report_content(aeat_720_report: &str) {
    let aeat_720_test_form = common::load_test_file(
        &(env!("CARGO_MANIFEST_DIR").to_owned() + "/tests/data/fichero-720_2019.txt"),
    )
    .expect("aeat 720 test file should exist");
    common::compare_strs_by_line(aeat_720_report, &aeat_720_test_form);
}

#[tokio::test]
#[ignore]
async fn test_all_reports_2019_chrome() -> WebDriverResult<()> {
    common::setup();
    let mut caps = DesiredCapabilities::chrome();
    caps.set_headless()?;

    let driver = WebDriver::new("http://localhost:4444", &caps).await?;

    let result = fill_all_fields(&driver).await;

    driver.quit().await?;

    if let Ok(aeat_720_form) = result {
        check_report_content(&aeat_720_form);
    } else {
        panic!("Unable to obtain AEAT 720 form");
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_all_reports_2019_firefox() -> WebDriverResult<()> {
    common::setup();
    let mut caps = DesiredCapabilities::firefox();
    caps.set_headless()?;

    let driver = WebDriver::new("http://localhost:4444", &caps).await?;

    let result = fill_all_fields(&driver).await;

    driver.quit().await?;

    if let Ok(aeat_720_form) = result {
        check_report_content(&aeat_720_form);
    } else {
        panic!("Unable to obtain AEAT 720 form and/or D6 aforix form");
    }

    Ok(())
}

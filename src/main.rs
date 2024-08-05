use inline_colorization::*;
use serde::{Deserialize, Serialize};
use simple_xml_builder::XMLElement;
use std::{
    io::{self, BufRead, Write},
    process::exit,
};

#[derive(Debug, Deserialize)]
enum DartTestLineType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "suite")]
    Suite,
    #[serde(rename = "testStart")]
    TestStart,
    #[serde(rename = "allSuites")]
    AllSuites,
    #[serde(rename = "testDone")]
    TestDone,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "print")]
    Print,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "done")]
    Done,
}

#[derive(Debug, Deserialize)]
struct DartQualifier {
    #[serde(rename = "type")]
    qualifier_type: DartTestLineType,
}

#[derive(Debug, Deserialize)]
struct Suite {
    id: i64,
    path: String,
    platform: String,
}

#[derive(Debug, Deserialize)]
struct TestStart {
    id: i64,

    name: String,

    #[serde(rename = "suiteID")]
    suite_id: i64,

    #[serde(rename = "groupIDs")]
    groupe_ids: Vec<i64>,

    root_url: Option<String>,

    root_column: Option<i64>,

    root_line: Option<i64>,

    line: Option<i64>,

    column: Option<i64>,

    result: Option<TestResult>,

    #[serde(skip)]
    prints: Vec<String>,

    #[serde(skip)]
    error: Option<String>,

    #[serde(skip)]
    stack_trace: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestStartContainer {
    test: TestStart,
    time: i64,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
enum TestResult {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "failure")]
    Failure,
}

#[derive(Debug, Deserialize)]
struct TestDoneContainer {
    #[serde(rename = "testID")]
    id: i64,

    result: TestResult,
}

#[derive(Debug, Deserialize)]
struct TestGroup {
    id: i64,
    #[serde(rename = "suiteID")]
    suite_id: i64,
    #[serde(rename = "parentID")]
    parent_id: Option<i64>,
    name: String,
    #[serde(rename = "testCount")]
    test_count: i64,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestGroupContainer {
    time: i64,
    group: TestGroup,
}

#[derive(Debug, Deserialize)]
struct SuiteContainer {
    time: i64,
    suite: Suite,

    #[serde(skip)]
    tests: Vec<TestStartContainer>,
}

#[derive(Debug, Deserialize)]
struct TestPrint {
    #[serde(rename = "testID")]
    test_id: i64,
    #[serde(rename = "messageType")]
    message_type: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct TestError {
    #[serde(rename = "testID")]
    test_id: i64,
    error: String,
    #[serde(rename = "stackTrace")]
    stack_trace: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestInfo {
    #[serde(rename = "test-name")]
    test_name: String,
}

impl PartialEq for SuiteContainer {
    fn eq(&self, other: &Self) -> bool {
        self.suite.id == other.suite.id
    }
}

fn main() {
    let stdin = io::stdin();
    let handle = stdin.lock();
    let reader = io::BufReader::new(handle);
    let mut suites = vec![];
    let mut tests = vec![];
    let mut groups = vec![];

    for line in reader.lines() {
        match line {
            Ok(content) => {
                if let Ok(test_line) = serde_json::from_str::<DartQualifier>(&content) {
                    match test_line.qualifier_type {
                        DartTestLineType::Suite => {
                            let suite: SuiteContainer = serde_json::from_str(&content).unwrap();
                            suites.push(suite);
                        }

                        DartTestLineType::TestStart => {
                            let test_start: TestStartContainer =
                                serde_json::from_str(&content).unwrap();
                            tests.push(test_start);
                        }

                        DartTestLineType::TestDone => {
                            let test_done: TestDoneContainer =
                                serde_json::from_str(&content).unwrap();

                            for test in tests.iter_mut() {
                                if test_done.id == test.test.id {
                                    test.test.result = Some(test_done.result);
                                    break;
                                }
                            }
                        }

                        DartTestLineType::Group => {
                            let group: TestGroupContainer = serde_json::from_str(&content).unwrap();
                            groups.push(group);
                        }

                        DartTestLineType::Print => {
                            let test_print: TestPrint = serde_json::from_str(&content).unwrap();
                            for test in tests.iter_mut() {
                                if test_print.test_id == test.test.id {
                                    test.test.prints.push(test_print.message.clone());
                                    break;
                                }
                            }
                        }

                        DartTestLineType::Error => {
                            let test_error: TestError = serde_json::from_str(&content).unwrap();
                            for test in tests.iter_mut() {
                                if test_error.test_id == test.test.id {
                                    test.test.error = Some(test_error.error.clone());
                                    if let Some(stack) = test_error.stack_trace {
                                        test.test.stack_trace = Some(stack);
                                    }
                                    break;
                                }
                            }
                        }

                        DartTestLineType::Start
                        | DartTestLineType::AllSuites
                        | DartTestLineType::Done => {
                            // Do nothing
                        }
                    }
                } else {
                    println!("{}", content);
                }
            }
            Err(err) => {
                eprintln!("Error reading line: {}", err);
                exit(1);
            }
        }
    }

    let current_dir = std::env::current_dir()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    for suite in &mut suites {
        let mut i = 0;
        let mut suite_elem = XMLElement::new("testsuite");
        suite_elem.add_attribute("file", &suite.suite.path);

        let suite_path = suite
            .suite
            .path
            .clone()
            .replace(&format!("{}/test/", current_dir), "");

        let output_path = format!(
            "{}/coverage/{}",
            current_dir.clone(),
            suite_path.clone().replace("/", "_"),
        );

        let suite_name = suite_path.split("/").last().unwrap();

        suite_elem.add_attribute("name", &suite_name);

        while i < tests.len() {
            if suite.suite.id == tests[i].test.suite_id {
                suite.tests.push(tests.remove(i));
            } else {
                i += 1;
            }
        }

        for test in &suite.tests {
            let mut test_case_elem = XMLElement::new("testcase");
            test_case_elem.add_attribute("name", &test.test.name);
            let time_in_minutes = test.time as f64 / 1000.0;
            test_case_elem.add_attribute("time", time_in_minutes.to_string());

            match test.test.result {
                Some(TestResult::Success) => {
                    // Do nothing
                }

                Some(TestResult::Error) | Some(TestResult::Failure) => {
                    let mut error_elem = XMLElement::new("error");
                    let mut msg = vec![];

                    if let Some(stack) = test.test.stack_trace.clone() {
                        msg.push(stack.trim().to_string());
                    }

                    for print in &test.test.prints {
                        msg.push(print.clone().trim().to_string());
                    }

                    error_elem.add_attribute("message", msg.join("\n"));
                    error_elem.add_text(test.test.error.as_ref().unwrap());
                    test_case_elem.add_child(error_elem);
                    let line = test.test.root_line.unwrap_or(test.test.line.unwrap_or(0));
                    let column = test
                        .test
                        .root_column
                        .unwrap_or(test.test.column.unwrap_or(0));

                    let error_msg = format!(
                        "{bg_red}{color_white} ✗ FAIL {color_reset}{bg_reset}{bg_white} {color_bright_black}{style_underline}{}:{}:{}{style_reset}{color_reset}{bg_reset}\n{}\n\n\t{}\n\n\t{color_red}{}{color_reset}\n\n",
                        suite_path,
                        line.to_string().trim(),
                        column.to_string().trim(),
                        format!("{color_blue}{}{color_reset}", test.test.name),
                        msg.join("\n").replace("\n", "\n\t").trim(),
                        test.test.error.as_ref().unwrap().trim().replace("\n", "\n\t").trim(),
                    );

                    io::stderr().write_all(error_msg.as_bytes()).unwrap();
                    io::stdout().flush().expect("Error flushing stdout");
                }
                None => {
                    let mut error_elem = XMLElement::new("error");
                    error_elem.add_attribute("message", "Test not finished");
                    test_case_elem.add_child(error_elem);
                }
            }

            for print in &test.test.prints {
                // Check for file paths in the prints. The accepted format is Golden "FILE.png":
                let golden_re = regex::Regex::new(r#"Golden "(.+?)""#).unwrap();
                let failures_path_re = regex::Regex::new(r#"(.*)/failures"#).unwrap();

                if let Some(_file_cap) = golden_re.captures(print) {
                    if let Some(folder_cap) = failures_path_re.captures(print) {
                        let directory = format!("{}/failures", folder_cap.get(1).unwrap().as_str());

                        // read files in directory
                        if let Ok(entries) = std::fs::read_dir(directory) {
                            let mut attachments = XMLElement::new("attachments");

                            // create recursive suite path
                            std::fs::create_dir_all(output_path.clone()).unwrap();

                            for entry in entries {
                                if let Ok(entry) = entry {
                                    let full_file_path = entry.path();
                                    // copy to coverage folder
                                    let file_name =
                                        full_file_path.file_name().unwrap().to_string_lossy();
                                    let new_file_path =
                                        format!("{}/{}", output_path.clone(), file_name,);

                                    std::fs::copy(&full_file_path, &new_file_path).unwrap();

                                    let mut attachment = XMLElement::new("attachment");
                                    attachment.add_text(file_name);
                                    attachments.add_child(attachment);
                                }
                            }
                            test_case_elem.add_child(attachments);
                        }
                    }
                }
            }

            if test.test.result == Some(TestResult::Success)
                && (test.test.name.starts_with("loading /")
                    || test.test.name.ends_with("(setUpAll)")
                    || test.test.name.ends_with("(setUp)")
                    || test.test.name.ends_with("(tearDownAll)")
                    || test.test.name.ends_with("(tearDown)"))
            {
                continue;
            }
            suite_elem.add_child(test_case_elem);
        }

        std::fs::create_dir_all(output_path.clone()).unwrap();

        let mut file =
            std::fs::File::create(format!("{}/results.xml", output_path.clone())).unwrap();
        file.write_all(suite_elem.to_string().as_bytes()).unwrap();

        let test_info = TestInfo {
            test_name: suite_name.to_string(),
        };

        let mut file =
            std::fs::File::create(format!("{}/test-info.json", output_path.clone())).unwrap();
        file.write_all(serde_json::to_string_pretty(&test_info).unwrap().as_bytes())
            .unwrap();
    }
}

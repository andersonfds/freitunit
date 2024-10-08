use clap::command;
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
struct DartEvent {
    event: String,
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
    #[serde(skip)]
    time_started: Option<i64>,
    #[serde(skip)]
    time_end: Option<i64>,
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

    time: i64,
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
    let mut command = command!()
        .about("Converts Dart test output to JUnit XML format")
        .override_usage("flutter test --machine | freitunit");

    if atty::is(atty::Stream::Stdin) {
        let _ = command.print_help();
        exit(1);
    }

    command.get_matches();

    let stdin = io::stdin();
    let handle = stdin.lock();
    let reader = io::BufReader::new(handle);
    let mut suites = vec![];
    let mut tests = vec![];
    let mut groups = vec![];
    let mut test_start_time = chrono::Utc::now();

    for line in reader.lines() {
        match line {
            Ok(content) => {
                let content = content.trim();
                if let Ok(test_line) = serde_json::from_str::<DartQualifier>(&content) {
                    match test_line.qualifier_type {
                        DartTestLineType::Suite => {
                            let suite: SuiteContainer = serde_json::from_str(&content).unwrap();
                            suites.push(suite);
                        }

                        DartTestLineType::TestStart => {
                            let mut test_start: TestStartContainer =
                                serde_json::from_str(&content).unwrap();
                            test_start.time_started = Some(chrono::Utc::now().timestamp_millis());
                            tests.push(test_start);
                        }

                        DartTestLineType::TestDone => {
                            let test_done: TestDoneContainer =
                                serde_json::from_str(&content).unwrap();

                            for test in tests.iter_mut() {
                                if test_done.id == test.test.id {
                                    test.test.result = Some(test_done.result);
                                    test.time_end = Some(test_done.time);
                                    test.print_failed();
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

                        DartTestLineType::Start => {
                            test_start_time = chrono::Utc::now();
                        }

                        DartTestLineType::AllSuites | DartTestLineType::Done => {
                            io::stdout().flush().expect("Error flushing stdout");
                        }
                    }
                } else if let Ok(_) = serde_json::from_str::<Vec<DartEvent>>(&content) {
                } else {
                    if !content.trim().is_empty() {
                        println!("{}", content);
                    }
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
        suite_elem.add_attribute("xmlns:xsi", "https://www.w3.org/2001/XMLSchema-instance");
        suite_elem.add_attribute("xsi:noNamespaceSchemaLocation", "https://github.com/jenkinsci/xunit-plugin/raw/master/src/main/resources/org/jenkinsci/plugins/xunit/types/model/xsd/junit-10.xsd");
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
        let mut errors = 0;
        let mut failures = 0;
        let mut tests = 0;

        for test in &suite.tests {
            let mut test_case_elem = XMLElement::new("testcase");
            test_case_elem.add_attribute("name", &test.test.name);
            let time_test_start = test_start_time + chrono::Duration::milliseconds(test.time);
            let time_test_end = test_start_time + chrono::Duration::milliseconds(test.time_end.unwrap());
            let duration = time_test_end - time_test_start;
            test_case_elem.add_attribute("timestamp", time_test_start.format("%Y-%m-%dT%H:%M:%S").to_string());
            test_case_elem.add_attribute("time", duration.num_milliseconds() as f64 / 1000.0);

            match test.test.result {
                Some(TestResult::Success) => {
                    tests += 1;
                }

                Some(TestResult::Error) | Some(TestResult::Failure) => {
                    tests += 1;
                    match test.test.result {
                        Some(TestResult::Error) => {
                            errors += 1;
                        }

                        Some(TestResult::Failure) => {
                            failures += 1;
                        }

                        _ => {}
                    }

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

        suite_elem.add_attribute("tests",  tests.to_string());
        suite_elem.add_attribute("errors", errors.to_string());
        suite_elem.add_attribute("failures", failures.to_string());
        suite_elem.add_attribute("timestamp", test_start_time.format("%Y-%m-%dT%H:%M:%S").to_string());

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

    let exit_code = if suites.iter().any(|suite| {
        suite
            .tests
            .iter()
            .any(|test| test.test.result == Some(TestResult::Error))
    }) {
        1
    } else {
        0
    };

    exit(exit_code);
}

impl TestStartContainer {
    fn print_failed(&self) {
        if self.test.result == Some(TestResult::Error)
            || self.test.result == Some(TestResult::Failure)
        {
            let current_dir = std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let suite_path = self
                .test
                .root_url
                .clone()
                .unwrap_or("".to_string())
                .replace("file://", "")
                .replace(&format!("{}/", &current_dir), "");

            let mut msg = vec![];

            if let Some(stack) = self.test.stack_trace.clone() {
                msg.push(stack.trim().to_string());
            }

            for print in &self.test.prints {
                msg.push(print.clone().trim().to_string());
            }

            let line = self.test.root_line.unwrap_or(self.test.line.unwrap_or(0));
            let column = self
                .test
                .root_column
                .unwrap_or(self.test.column.unwrap_or(0));

            let error_msg = format!(
                "{bg_red}{color_white} ✗ FAIL {color_reset}{bg_reset}{bg_white} {color_bright_black}{style_underline}{}:{}:{}{style_reset}{color_reset}{bg_reset}\n{}\n\n\t{}\n\n\t{color_red}{}{color_reset}\n\n",
                suite_path,
                line.to_string().trim(),
                column.to_string().trim(),
                format!("{color_blue}{}{color_reset}", self.test.name),
                msg.join("\n").replace("\n", "\n\t").trim(),
                self.test.error.as_ref().unwrap().trim().replace("\n", "\n\t").trim(),
            );

            io::stderr().write_all(error_msg.as_bytes()).unwrap();
            io::stderr().flush().unwrap();
        }
    }
}

use std::{
    io::{self, BufRead},
    process::exit,
};

use serde::Deserialize;
use simple_xml_builder::XMLElement;


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

    result: Option<TestResult>,

    #[serde(skip)]
    prints: Vec<String>,

    #[serde(skip)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TestStartContainer {
    test: TestStart,
    time: i64,
}

#[derive(Debug, Deserialize)]
enum TestResult {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
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
                let test_line: DartQualifier = serde_json::from_str(&content).unwrap();

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
                        let test_done: TestDoneContainer = serde_json::from_str(&content).unwrap();

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
                                break;
                            }
                        }
                    }

                    DartTestLineType::Start | DartTestLineType::AllSuites => {
                        // Do nothing
                    }

                    _ => {
                        println!("{}", content);
                    }
                }
            }
            Err(err) => {
                exit(1);
            }
        }
    }

    for suite in &mut suites {
        let mut i = 0;
        let mut suite_elem = XMLElement::new("testsuite");
        suite_elem.add_attribute("file", &suite.suite.path);
        suite_elem.add_attribute("name", &suite.suite.path.replace("/", "."));

        let mut test_case_elem = XMLElement::new("testcase");

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
            test_case_elem.add_attribute("time", &test.time.to_string());

            match test.test.result {
                Some(TestResult::Success) => {
                    // Do nothing
                }
                Some(TestResult::Error) => {
                    let mut error_elem = XMLElement::new("error");
                    error_elem.add_attribute("message", &test.test.error.as_ref().unwrap());
                    test_case_elem.add_child(error_elem);
                }
                None => {
                    let mut error_elem = XMLElement::new("error");
                    error_elem.add_attribute("message", "Test not finished");
                    test_case_elem.add_child(error_elem);
                }
            }

            for print in &test.test.prints {
                let mut system_out_elem = XMLElement::new("system-out");
                system_out_elem.add_text(print);
                test_case_elem.add_child(system_out_elem);
            }

            suite_elem.add_child(test_case_elem);
        }

        println!("{}", suite_elem.to_string());
    }
}

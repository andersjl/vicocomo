pub fn test_crate(crate_dir: &str, show_stdout: bool) {
    use ::std::process::Command;

    let output = Command::new("cargo")
        .current_dir(crate_dir)
        .arg("run")
        .arg("--quiet")
        .output()
        .expect(&format!("*** 'cd {}; cargo run' failed", crate_dir));
    let exit_code = output.status.code();
    if exit_code.is_none() || exit_code.unwrap() != 0 {
        if show_stdout {
            eprintln!(
                "tested program stdout: {}",
                String::from_utf8(output.stdout)
                    .unwrap_or(String::new())
                    .as_str(),
            );
        }
        eprintln!(
            "tested program stderr: {}",
            String::from_utf8(output.stderr)
                .unwrap_or(String::new())
                .as_str(),
        );
        panic!(
            "tested program exit code: {}",
            exit_code.unwrap_or(-1111111111)
        );
    }
}

#[macro_export]
macro_rules! test_http_server {
    ($server_path:expr, $(  $request:expr, $test:expr),* $(,)?) => {
        {
            let mut reqs = Vec::new();
        $(  reqs.push($request); )*
            let mut resps =
                $crate::test_utils::run_http_server($server_path, reqs);
        $(  $test(&resps.drain(..1).next().unwrap()); )*
            println!("OK");
        }
    };
}

pub fn run_http_server(
    server_dir: &str,
    requests: Vec<TestRequest>,
) -> Vec<TestResponse> {
    use ::std::process::Command;
    use ::std::thread::sleep;
    use ::std::time::Duration;

    let mut result: Vec<TestResponse> = Vec::new();
    let mut cargo_build = Command::new("cargo");
    let mut cargo_run = Command::new("cargo");
    println!("building {} ... ", server_dir);
    if let Ok(mut build) = cargo_build
        .current_dir(server_dir)
        .arg("build")
        .arg("--quiet")
        .spawn()
    {
        build.wait().expect("*** cargo build was not running");
        println!("    OK");
    } else {
        println!("*** cargo build did not start");
    }
    println!("running {} ... ", server_dir);
    if let Ok(mut run) = cargo_run
        .current_dir(server_dir)
        .arg("run")
        .arg("--quiet")
        .spawn()
    {
        sleep(Duration::from_secs(1));
        for req in requests {
            result.push(req.fetch_response());
        }
        run.kill().unwrap();
    } else {
        println!("*** cargo run did not start");
    }
    result
}

#[derive(Clone, Debug)]
pub struct TestRequest {
    url: String,
    data: Vec<String>,
    get: bool,
    redir: bool,
    headers: Vec<String>,
}

impl TestRequest {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            data: Vec::new(),
            get: true,
            redir: true,
            headers: Vec::new(),
        }
    }

    pub fn data(mut self, name: &str, value: &str) -> Self {
        self.data.push(format!("{}={}", name, value));
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push(format!("{}: {}", name, value));
        self
    }

    pub fn no_redirect(mut self) -> Self {
        self.redir = false;
        self
    }

    pub fn fetch_response(self) -> TestResponse {
        use ::lazy_static::lazy_static;
        use ::regex::Regex;
        lazy_static! {
            static ref OUTPUT: Regex = Regex::new(
                r#"((?s).*)"\n__variables__\nstatus: (.*)\nredirect: (.*)""#,
            ).unwrap();
        }
        use ::std::process::Command;
        let mut curl = Command::new("curl");
        curl.arg("--silent");
        for header in self.headers {
            curl.arg("--header").arg(&format!("\"{}\"", header));
        }
        curl.arg("--write-out").arg(
            "\"\
                \n__variables__\
                \nstatus: %{response_code}\
                \nredirect: %{redirect_url}\
            \"",
        );
        if self.get {
            curl.arg("--get");
        }
        if self.redir {
            curl.arg("--location");
        }
        for item in self.data {
            curl.arg("--data-urlencode").arg(&format!("\"{}\"", item));
        }
        let output = String::from_utf8(
            curl.arg(&self.url).output().unwrap().stdout,
        ).unwrap();
        let parts = OUTPUT.captures(&output).unwrap();
        TestResponse {
            body: parts.get(1).unwrap().as_str().to_string(),
            status: parts.get(2).unwrap().as_str().to_string(),
            redirect: parts.get(3).unwrap().as_str().to_string(),
        }
    }

    pub fn post(mut self, name: &str, value: &str) -> Self {
        self.get = false;
        self.data(name, value)
    }
}

#[derive(Clone, Debug)]
pub struct TestResponse {
    body: String,
    redirect: String,
    status: String,
}

impl TestResponse {
    pub fn body(&self) -> &str {
        self.body.as_str()
    }

    pub fn redirect(&self) -> &str {
        self.redirect.as_str()
    }

    pub fn status(&self) -> &str {
        self.status.as_str()
    }
}


# A pure-HTML front-end for StackMuncher

This app is used as a Lambda function for rendering a static HTML representation of StackMuncher public DB.

JSON data is retrieved from ElasticSearch and rendered by [Tera](https://tera.netlify.app/docs/) using [embedded templates](https://crates.io/crates/rust-embed). Tera is [a bit slow](https://github.com/djc/template-benchmarks-rs), but was chosen for its simple and powerful templating language. The average processing time is between 500 - 2000ms per page, which is a lot for a 100KB page.

This is a stop-gap solution to get something simple out quickly. Better templating and more parallelized queries should be used in the future.

## Lambda deployment

Create a function called `stm_html_ui` with `stm-www` role, a custom runtime and customize these settings:
* env vars: see [config.rs](./src/config.rs) for the full list
* timeout: 30s
* reserved concurrency: 5
* async invocation: zero retries
****
```
cargo build --release --target x86_64-unknown-linux-gnu --package stm_html_ui
strip ./target/x86_64-unknown-linux-gnu/release/stm_html_ui
cp ./target/x86_64-unknown-linux-gnu/release/stm_html_ui ./bootstrap && zip stm_html_ui.zip bootstrap && rm bootstrap
aws lambda update-function-code --region us-east-1 --function-name stm_html_ui --zip-file fileb://stm_html_ui.zip
```

#### Authorizer

This lambda checks every request for `Authorization` header if `Authorization` env variable was set with a value. The processing goes ahead only if the header matches the env var value.

The standard approach for authorizing APIGW requests would be IAM or a separate authorizer function, but the only reason we need to restrict access is to make sure the GWAPI is called via CloudFront to enable caching and AWS WAF. Apparently, there is no way to include CloudFront in an APIGW Lambda policy and adding `Authorization` header to the CloudFront origin is next best option.

#### API Gateway

* HTTP API with Lambda
* ANY /{proxy+}
* `stm_html_iu` Lambda
* `$default` stage

## Debugging

This app relies on https://github.com/rimutaka/lambda-debug-proxy to run a local copy on your dev machine connected to the GatewayAPI via SQS.

1. Deploy https://github.com/rimutaka/lambda-debug-proxy in place of *stm_html_iu*.
2. Configure the request and response SQS queues to accept messages from *stm_html_iu* lambda and the client machine running the debugger.
3. Add `STM_HTML_LAMBDA_PROXY_REQ` and `STM_HTML_LAMBDA_PROXY_RESP` env vars with the queue URLs to your *.bashrc*.
4. Use `cargo run` to launch *stm_html_iu* app locally
5. Send a request to the GWAPI endpoint to invoke *stm_html_iu* 

The above steps should trigger a chain of requests and responses: 
> APIGW -> Lambda *stm_html_iu* proxy -> SQS Request Queue -> the locally run *stm_html_iu* app -> SQS Response Queue -> Lambda *stm_html_iu* proxy -> APIGW

[main.rs](./src/main.rs) has sections of code annotated with `#[cfg(debug_assertions)]` to use *lambda-debug-proxy* feature in DEBUG mode or exclude it when built with `--release`.

Sample debug output:

```
Feb 11 22:49:45.869  INFO stm_html::proxy: New msg
Feb 11 22:49:45.908  INFO stm_html::elastic: ES query 126 started
Feb 11 22:49:45.908  INFO stm_html::elastic: ES query 126 started
Feb 11 22:49:45.909  INFO stm_html::elastic: ES query 228 started
Feb 11 22:49:47.259  INFO stm_html::elastic: ES query 126 response arrived
Feb 11 22:49:47.260  INFO stm_html::elastic: ES query 126 finished
Feb 11 22:49:47.263  INFO stm_html::elastic: ES query 126 response arrived
Feb 11 22:49:47.263  INFO stm_html::elastic: ES query 126 finished
Feb 11 22:49:47.466  INFO stm_html::elastic: ES query 228 response arrived
Feb 11 22:49:47.674  INFO stm_html::elastic: ES query 228 finished
Feb 11 22:49:47.728  INFO stm_html::html::keyword: Rendered
Feb 11 22:49:49.180  INFO stm_html::proxy: Msg sent
```

Messages like *ES query 126 ...* refer to the same query where 126 is a simple hash of its content used to identify the query in the log stream. It does not tell us what the query actually is. 
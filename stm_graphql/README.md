# A GraphQL interface for StackMuncher Vue front-end

This app is used as a Lambda function for handling GraphQL requests from Vue front-end via API Gateway.

JSON data is retrieved from ElasticSearch and rendered by ...

## Lambda deployment

Create a function called `stm_graphql` with `stm-www` role, a custom runtime and customize these settings:
* env vars: see [config.rs](./src/config.rs) for the full list
* timeout: 30s
* reserved concurrency: 5
* async invocation: zero retries
****
```
cargo build --release --target x86_64-unknown-linux-gnu --package stm_graphql
strip ./target/x86_64-unknown-linux-gnu/release/stm_graphql
cp ./target/x86_64-unknown-linux-gnu/release/stm_graphql ./bootstrap && zip stm_graphql.zip bootstrap && rm bootstrap
aws lambda update-function-code --region us-east-1 --function-name stm_graphql --zip-file fileb://stm_graphql.zip
```

#### Authorizer

This lambda looks for a JWT in `Authorization` header and returns HTTP 401 error if it's missing, expired or invalid.

#### API Gateway

* HTTP API with Lambda
* ANY /{proxy+}
* `stm_graphql` Lambda
* `$default` stage
* CORS: configure all fields / headers in the form

## Debugging

This app relies on https://github.com/rimutaka/lambda-debug-proxy to run a local copy on your dev machine connected to the GatewayAPI via SQS.

1. Deploy https://github.com/rimutaka/lambda-debug-proxy in place of *stm_graphql*.
2. Configure the request and response SQS queues to accept messages from *stm_graphql* lambda and the client machine running the debugger.
3. Add `STM_GRAPHQL_PROXY_REQ` and `STM_GRAPHQL_PROXY_RESP` env vars with the queue URLs to your *.bashrc*.
4. Use `cargo run` to launch *stm_graphql* app locally
5. Send a request to the GWAPI endpoint to invoke *stm_graphql* 

The above steps should trigger a chain of requests and responses: 
> APIGW -> Lambda *stm_graphql* proxy -> SQS Request Queue -> the locally run *stm_graphql* app -> SQS Response Queue -> Lambda *stm_graphql* proxy -> APIGW

[main.rs](./src/main.rs) has sections of code annotated with `#[cfg(debug_assertions)]` to use *lambda-debug-proxy* feature in DEBUG mode or exclude it when built with `--release`.

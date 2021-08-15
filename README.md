# StackMuncher Server Project

StackMuncher Server processes stack reports submitted by devs and provides a UI for their search on https://stackmuncher.com. 

## List of Crates

* _stm_inbox_: accepts new report submissions, AWS Lambda
* _stm_inbox_router_: validates new submissions, assigns them to a developer account and queues up report processing jobs, AWS Lambda
* _stm_inbox_flows_: an app with multiple report handlers to process, re-process, delete or change format, runs on a VM
* _stm_html_ui_: a minimal UI front-end for displaying developer profiles

Each crate included in this project has its own ReadMe with architecture and deployment details.

See https://github.com/stackmuncher/stm_app for more info on the app making the submissions.
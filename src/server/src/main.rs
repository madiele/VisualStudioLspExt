// nvim attach snippet
// lua vim.lsp.buf_attach_client(0, vim.lsp.start_client {name = "test", cmd = {"./target/debug/server.exe"}})

use std::error::Error;

use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    request::{CodeActionRequest, ExecuteCommand, HoverRequest},
    CodeAction, CodeActionKind, CodeActionProviderCapability, CodeLensOptions, Command, Diagnostic,
    DiagnosticSeverity, ExecuteCommandOptions, Hover, HoverProviderCapability, InitializeParams,
    MarkedString, Position, PublishDiagnosticsParams, Range, ServerCapabilities, Url,
    WorkDoneProgressOptions,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("lps server starting");

    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec!["fake".to_string()],
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: Some(false),
            },
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    })
    .unwrap();

    let initialize_params = match connection.initialize(capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };

    let json_capabilities = serde_json::to_string(&initialize_params)?;

    eprintln!("client capabilities: {json_capabilities:?}");

    main_loop(connection, initialize_params)?;

    io_threads.join()?;

    eprintln!("stopping server");

    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    eprintln!("starting main loop");
    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");
        match msg {
            lsp_server::Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                eprintln!("got req: {req:?}");

                let req = match cast::<HoverRequest>(req) {
                    Ok((id, params)) => {
                        eprintln!("got HoverRequest: #{id}, {params:?}");
                        let result = Some(Hover {
                            contents: lsp_types::HoverContents::Scalar(MarkedString::String(
                                "hello world".to_string(),
                            )),
                            range: None,
                        });
                        let result_json = serde_json::to_value(&result).unwrap();
                        let response = Response {
                            id,
                            result: Some(result_json),
                            error: None,
                        };
                        connection.sender.send(Message::Response(response))?;
                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(req)) => req,
                };

                let req = match cast::<CodeActionRequest>(req) {
                    Ok((id, params)) => {
                        eprintln!("got CodeActionRequest: #{id}, {params:?}");
                        let uri = params.text_document.uri;
                        let result = Some(vec![CodeAction {
                            title: "test code action".to_string(),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: None,
                            edit: None,
                            command: Some(Command {
                                title: "fakediagnostics".to_string(),
                                command: "fake".to_string(),
                                arguments: Some(vec![json!(uri)]),
                            }),
                            is_preferred: Some(false),
                            disabled: None,
                            data: None,
                        }]);
                        let result_json = serde_json::to_value(&result).unwrap();
                        let response = Response {
                            id,
                            result: Some(result_json),
                            error: None,
                        };
                        connection.sender.send(Message::Response(response))?;
                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(req)) => req,
                };

                match cast::<ExecuteCommand>(req) {
                    Ok((id, mut params)) => {
                        eprintln!("got ExecuteCommand: #{id}, {params:?}");
                        let uri = serde_json::from_value::<String>(params.arguments[0].take())?;
                        let result = Some(lsp_types::LSPAny::default());
                        let result_json = serde_json::to_value(result).unwrap();
                        let response = Response {
                            id,
                            result: Some(result_json),
                            error: None,
                        };
                        connection.sender.send(Message::Response(response))?;
                        let notification = Notification::new(
                            "textDocument/publishDiagnostics".to_string(),
                            PublishDiagnosticsParams {
                                uri: Url::parse(uri.as_str())?,
                                diagnostics: vec![Diagnostic {
                                    range: Range {
                                        start: Position {
                                            line: 1,
                                            character: 1,
                                        },
                                        end: Position {
                                            line: 1,
                                            character: 2,
                                        },
                                    },
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    code: None,
                                    code_description: None,
                                    source: None,
                                    message: "lol".to_string(),
                                    related_information: None,
                                    tags: None,
                                    data: None,
                                }],
                                version: None,
                            },
                        );
                        connection
                            .sender
                            .send(Message::Notification(notification))?;
                        continue;
                    }
                    Err(err @ ExtractError::JsonError { .. }) => panic!("{err:?}"),
                    Err(ExtractError::MethodMismatch(req)) => req,
                };
            }
            lsp_server::Message::Response(res) => {
                eprintln!("got response: {res:?}");
            }
            lsp_server::Message::Notification(not) => {
                eprintln!("got notification: {not:?}");
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};
    use tree_sitter::{Parser, Query, QueryCursor};
    use tree_sitter_c_sharp::language;

    #[test]
    fn test_parse_sample_cs() {
        // Read the sample.cs file
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_owned();
        path.push("client");
        path.push("vs2022");
        path.push("VSIXProject1");
        path.push("Client.cs");

        let source_code = fs::read_to_string(path).expect("Unable to read file");

        // Create a parser
        let mut parser = Parser::new();
        parser
            .set_language(language())
            .expect("Error setting language");

        // Parse the source code
        let tree = parser
            .parse(&source_code, None)
            .expect("Error parsing code");

        // Create a query
        let query_source = "(class_declaration (identifier) @class)";
        //let query_source = r#"
        //                            (class_declaration
        //                            name: (identifier) @controller (#match? @controller ".*Controller")
        //                            bases: (base_list (identifier) @interface) (#match? @interface ".*Base")
        //                            )
        //"#;
        let query = Query::new(language(), query_source).expect("Error creating query");

        // Perform the query
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source_code.as_bytes());

        // Print the matched classes
        for mat in matches {
            for cap in mat.captures {
                let node = cap.node;
                let class_name = source_code[node.start_byte()..node.end_byte()].to_string();
                let start = node.start_position();
                let end = node.end_position();
                println!("Matched class: {class_name} range: {start:?} -> {end:?}");
            }
        }
    }
}

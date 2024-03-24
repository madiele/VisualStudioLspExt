// nvim attach snippet
// lua vim.lsp.buf_attach_client(0, vim.lsp.start_client {name = "test", cmd = {"./target/debug/server.exe"}})

use std::error::Error;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    request::{CodeActionRequest, HoverRequest},
    CodeAction, CodeActionKind, CodeActionProviderCapability, CodeLensOptions, Command, Hover,
    HoverProviderCapability, InitializeParams, MarkedString, ServerCapabilities,
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("lps server starting");

    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
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

                match cast::<CodeActionRequest>(req) {
                    Ok((id, params)) => {
                        eprintln!("got CodeActionRequest: #{id}, {params:?}");
                        let result = Some(vec![CodeAction {
                            title: "test code action".to_string(),
                            kind: Some(CodeActionKind::REFACTOR),
                            diagnostics: None,
                            edit: None,
                            command: Some(Command {
                                title: "TODO".to_string(),
                                command: "TODO".to_string(),
                                arguments: None,
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

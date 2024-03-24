use std::error::Error;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    request::GotoDefinition, GotoDefinitionResponse, InitializeParams, OneOf, ServerCapabilities,
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("lps server starting");

    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
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

                match cast::<GotoDefinition>(req) {
                    Ok((id, params)) => {
                        eprintln!("got GotoDefinition: #{id}, {params:?}");

                        let result = Some(GotoDefinitionResponse::Array(Vec::new()));
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

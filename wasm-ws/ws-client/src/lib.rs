use js_sys::{Error, Promise};
use wasm_bindgen::{prelude::*, JsValue};
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

#[wasm_bindgen(js_name= wsPing)]
pub fn ws_ping(endpoint: &str, message: &str) -> Promise {
    js_sys::Promise::new(&mut |resolve, reject| {
        // Connect to the endpoint
        let ws = WebSocket::new(endpoint).unwrap();

        // Create onopen callback
        let cloned_reject = reject.clone();
        let cloned_ws = ws.clone();
        let cloned_message = message.to_string();
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            if let Err(err) = cloned_ws.send_with_str(&cloned_message) {
                cloned_reject
                    .call1(&JsValue::NULL, &err)
                    .expect("call to reject shouldn't fail");
            }
        });
        // Set onopen event handler on websocket
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        // Forget the callback to keep it alive
        onopen_callback.forget();

        // Create onerror callback
        let cloned_reject = reject.clone();
        let cloned_ws = ws.clone();
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            // We close the connection silently
            let _ = cloned_ws.close();
            cloned_reject
                .call1(&JsValue::NULL, &e.error())
                .expect("call to reject shouldn't fail");
        });
        // Set onerror event handler on websocket
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        // Forget the callback to keep it alive
        onerror_callback.forget();

        // Create onmessage callback
        let cloned_ws = ws.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // We close the connection silently
            let _ = cloned_ws.close();
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                resolve
                    .call1(&JsValue::NULL, &txt)
                    .expect("call to resolve shouldn't fail");
            } else {
                reject
                    .call1(
                        &JsValue::NULL,
                        &Error::new("received unsupported message type"),
                    )
                    .expect("call to reject shouldn't fail");
            }
        });
        // Set onmessage event handler on websocket
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // Forget the callback to keep it alive
        onmessage_callback.forget();
    })
}

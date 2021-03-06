// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
use super::dispatch_json::{JsonOp, Value};
use crate::op_error::OpError;
use crate::ops::json_op;
use crate::state::State;
use crate::web_worker::WebWorkerHandle;
use crate::worker::WorkerEvent;
use deno_core::*;
use futures::channel::mpsc;
use std::convert::From;

pub fn web_worker_op<D>(
  sender: mpsc::Sender<WorkerEvent>,
  dispatcher: D,
) -> impl Fn(
  &mut deno_core::Isolate,
  Value,
  Option<ZeroCopyBuf>,
) -> Result<JsonOp, OpError>
where
  D: Fn(
    &mpsc::Sender<WorkerEvent>,
    Value,
    Option<ZeroCopyBuf>,
  ) -> Result<JsonOp, OpError>,
{
  move |_isolate: &mut deno_core::Isolate,
        args: Value,
        zero_copy: Option<ZeroCopyBuf>|
        -> Result<JsonOp, OpError> { dispatcher(&sender, args, zero_copy) }
}

pub fn web_worker_op2<D>(
  handle: WebWorkerHandle,
  sender: mpsc::Sender<WorkerEvent>,
  dispatcher: D,
) -> impl Fn(
  &mut deno_core::Isolate,
  Value,
  Option<ZeroCopyBuf>,
) -> Result<JsonOp, OpError>
where
  D: Fn(
    WebWorkerHandle,
    &mpsc::Sender<WorkerEvent>,
    Value,
    Option<ZeroCopyBuf>,
  ) -> Result<JsonOp, OpError>,
{
  move |_isolate: &mut deno_core::Isolate,
        args: Value,
        zero_copy: Option<ZeroCopyBuf>|
        -> Result<JsonOp, OpError> {
    dispatcher(handle.clone(), &sender, args, zero_copy)
  }
}

pub fn init(
  i: &mut Isolate,
  s: &State,
  sender: &mpsc::Sender<WorkerEvent>,
  handle: WebWorkerHandle,
) {
  i.register_op(
    "op_worker_post_message",
    s.core_op(json_op(web_worker_op(
      sender.clone(),
      op_worker_post_message,
    ))),
  );
  i.register_op(
    "op_worker_close",
    s.core_op(json_op(web_worker_op2(
      handle,
      sender.clone(),
      op_worker_close,
    ))),
  );
}

/// Post message to host as guest worker
fn op_worker_post_message(
  sender: &mpsc::Sender<WorkerEvent>,
  _args: Value,
  data: Option<ZeroCopyBuf>,
) -> Result<JsonOp, OpError> {
  let d = Vec::from(data.unwrap().as_ref()).into_boxed_slice();
  let mut sender = sender.clone();
  sender
    .try_send(WorkerEvent::Message(d))
    .expect("Failed to post message to host");
  Ok(JsonOp::Sync(json!({})))
}

/// Notify host that guest worker closes
fn op_worker_close(
  handle: WebWorkerHandle,
  sender: &mpsc::Sender<WorkerEvent>,
  _args: Value,
  _data: Option<ZeroCopyBuf>,
) -> Result<JsonOp, OpError> {
  let mut sender = sender.clone();
  // Notify parent that we're finished
  sender.close_channel();
  // Terminate execution of current worker
  handle.terminate();
  Ok(JsonOp::Sync(json!({})))
}

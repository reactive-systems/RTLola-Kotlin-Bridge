use core::time::Duration;
use jni::errors::Result;
use jni::objects::{JObject, JString};
use jni::strings::JavaStr;
use jni::sys::{jbooleanArray, jdouble, jdoubleArray, jint, jlong, jstring};
use jni::JNIEnv;
use ordered_float::NotNan;
use rtlola_frontend::ParserConfig;
use rtlola_interpreter::{EvalConfig, Incremental, Monitor, TimeFormat, TimeRepresentation, Value};
use std::ffi::CStr;

mod bridge;

static mut MONITOR: Option<bridge::KotlinMonitor> = None;

//Function to initialize the Monitor we will feed with events, with the RDE-Specification
#[no_mangle]
pub unsafe extern "C" fn Java_de_unisaarland_loladrives_Sinks_RDEValidator_initmonitor(
    env: JNIEnv,
    o: JObject,
    j_recipient: JString,
    relevant_outputs: JString
) -> jstring {
    let m = bridge::init(env, o, j_recipient, relevant_outputs);
    MONITOR = Some(m);
    let output = env.new_string("Worked".to_owned()).unwrap();
    output.into_inner()
}


//Function which transmits the new Values of the current Period (1Hz) to the Monitor and returns the generated outputs to the App
//6 Float64 Input Streams (Float64) and 1 Trigger
#[no_mangle]
pub unsafe extern "C" fn Java_de_unisaarland_loladrives_Sinks_RDEValidator_sendevent(
    env: JNIEnv,
    o: JObject,
    inputs: jdoubleArray,
) -> jdoubleArray {
    let a = MONITOR.as_mut().unwrap();
    let res = bridge::receive_total_event(env, o, a, inputs);
    res
}
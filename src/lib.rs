// #![cfg(target_os = "android")]
#![allow(non_snake_case)]

use core::time::Duration;
use jni::objects::{JObject, JString};
use jni::sys::{jdouble, jdoubleArray, jlong, jlongArray, jstring};
use jni::JNIEnv;
use ordered_float::NotNan;
use rtlola_frontend::mir::StreamReference;
use std::ffi::{CStr, CString};
use rtlola_frontend::{ParserConfig, RtLolaMir};
use rtlola_interpreter::{EvalConfig, Incremental, Monitor, TimeFormat, TimeRepresentation, Value};
static mut MONITOR: Option<Monitor> = None;
static mut IR: Option<RtLolaMir> = None;
static RELEVANT_OUTPUTS: [&str; 19] = ["d", "d_u", "d_r", "d_m", "t_u", "t_r", "t_m", "u_avg_v", "r_avg_v", "m_avg_v", "u_va_pct", "r_va_pct", "m_va_pct", "u_rpa", "r_rpa", "m_rpa", "nox_per_kilometer", "is_valid_test_num", "not_rde_test_num"];
static mut RELEVANT_OUTPUT_IX: [usize; 19] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

//Function to initialize the Monitor we will feed with events, with the RDE-Specification
#[no_mangle]
pub unsafe extern "C" fn Java_de_unisaarland_loladrives_Sinks_RDEValidator_initmonitor(
    env: JNIEnv,
    _: JObject,
    j_recipient: JString,
) -> jstring {
    let spec_file = CString::from(CStr::from_ptr(
        env.get_string(j_recipient).unwrap().as_ptr(),
    ));

    let pc = ParserConfig::for_string(spec_file.to_str().unwrap().to_string());
    let ir = rtlola_frontend::parse(pc).unwrap();

    let ec = EvalConfig::api(TimeRepresentation::Relative(TimeFormat::HumanTime));
    let m : Monitor<Incremental> = rtlola_interpreter::Config::new_api(ec, ir.clone()).as_api();

    let indices: Vec<usize> = RELEVANT_OUTPUTS
        .iter()
        .map(|name| {
            let r = ir
                .outputs
                .iter()
                .find(|o| &o.name == *name)
                .expect("ir does not contain required output stream")
                .reference;
            if let StreamReference::Out(r) = r {
                r
            } else {
                panic!("output stream has input stream reference")
            }
        })
        .collect();
    for i in 0..RELEVANT_OUTPUT_IX.len() {
        // should prob be a mem copy of sorts.
        RELEVANT_OUTPUT_IX[i] = indices[i];
    }
    assert_eq!(NUM_OUTPUTS, RELEVANT_OUTPUTS.len());
    IR = Some(ir);
    MONITOR = Some(m);
    //Just to match the output-type, will remove this later
    let output = env.new_string("Worked".to_owned()).unwrap();
    output.into_inner()
    //----
}

//Function which transmits the new Values of the current Period (1Hz) to the Monitor and returns the generated outputs to the App
//6 Float64 Input Streams (Float64) and 1 Trigger
#[no_mangle]
pub unsafe extern "C" fn Java_org_rdeapp_pcdftester_Sinks_RDEValidator_sendevent(
    env: JNIEnv,
    _: JObject,
    inputs: jdoubleArray,
) -> jdoubleArray {
    // //jdouble = f64 (seems to work)
    let num_values = IR.as_ref().unwrap().inputs.len() + 1;
    let mut event = vec![0.0; num_values];
    let copy_res = env.get_double_array_region(inputs, 0, &mut event);

    debug_assert!(copy_res.is_ok());
    if copy_res.is_err() {
        let res = env.new_double_array(0).unwrap();
        return res;
    }

    let (time, input) = event.split_last().unwrap();
    let input: Vec<Value> = input
        .into_iter()
        .map(|f| Value::Float(NotNan::new(*f).unwrap()))
        .collect();
    let updates = MONITOR
        .as_mut()
        .unwrap()
        .accept_event(input, Duration::new(time.floor() as u64, 0));

    let num_updates = updates.timed.len();
    let res = env
        .new_double_array((num_updates * NUM_OUTPUTS) as i32)
        .unwrap();
    let output_copy_res: jni::errors::Result<()> = updates
        .timed
        .iter()
        .enumerate()
        .map(|(ix, update)| {
            let (_, values) = update;
            let output: Vec<jdouble> = values
                .iter()
                .filter_map(|(sr, v)| {
                    if RELEVANT_OUTPUT_IX.contains(sr) {
                        Some(v)
                    } else {
                        None
                    }
                })
                .map(|v| {
                    if let Value::Float(f) = v {
                        f.into_inner() as jdouble
                    } else {
                        0.0 as jdouble
                    }
                })
                .collect();
            env.set_double_array_region(res, (NUM_OUTPUTS * ix) as i32, &output)
        })
        .collect();
    debug_assert!(output_copy_res.is_ok());
    res
}

static NUM_OUTPUTS: usize = 19;

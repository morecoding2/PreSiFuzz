// SPDX-FileCopyrightText: 2022 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0

use libafl::{
    executors::{ExitKind},
    observers::{MapObserver, Observer},
    observers::{DifferentialObserver, ObserversTuple},
    Error,
    inputs::{UsesInput},
};

use core::{
    slice::from_raw_parts,
    fmt::Debug,
    slice::IterMut,
    slice::Iter,
};
use serde::{Deserialize, Serialize};
use libafl_bolts::{
    AsIter, AsIterMut, HasLen, Named,
};

use std::fs::File;
use std::io::prelude::*;
use flate2::bufread::GzDecoder;

extern crate fs_extra;
use std::{
    str,
    hash::Hasher,
};
use ahash::AHasher;
use std::io::Cursor;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum VerdiCoverageMetric {
    Line = 4,
    Toggle = 5,
    FSM = 6,
    Condition = 7,
    Branch = 8,
    Assert = 9,
}

/// A simple observer, just overlooking the runtime of the target.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[allow(clippy::unsafe_derive_deserialize)]
pub struct VerdiXMLMapObserver 
{
    name: String,
    map: Vec<u32>, 
    vdb: String,
    workdir: String,
    metric: VerdiCoverageMetric,
    filter: String
}

impl VerdiXMLMapObserver
{
    /// Creates a new [`MapObserver`]
    ///
    /// # Note
    /// Will get a pointer to the map and dereference it at any point in time.
    /// The map must not move in memory!
    #[must_use]
    pub fn new(name: &'static str, vdb: &String, workdir: &str, metric: VerdiCoverageMetric, filter: &String) -> Self {

        Self {
            name: name.to_string(),
            map: Vec::<u32>::new(),
            workdir: workdir.to_string(),
            metric,
            filter: filter.to_string(),
            vdb: vdb.to_string()
        }
    }
  
    /// Gets cnt as usize
    #[must_use]
    pub fn cnt(&self) -> usize {
        self.map.len()
    }
    
    /// Gets map ptr
    #[must_use]
    pub fn my_map(&self) -> &[u32] {
        self.map.as_slice()
    }
    
}

impl Named for VerdiXMLMapObserver
{
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasLen for VerdiXMLMapObserver
{
    fn len(&self) -> usize {
        self.map.len()
    }
}

impl MapObserver for VerdiXMLMapObserver
{
    type Entry = u32;

    #[inline]
    fn initial(&self) -> u32 {
       0 
    }

    #[inline]
    fn get(&self, idx: usize) -> &u32 {
        &self.map.as_slice()[idx]
    }

    #[inline]
    fn get_mut(&mut self, idx: usize) -> &mut u32 {
        &mut self.map.as_mut_slice()[idx]
    }

    /// Count the set bytes in the map
    fn count_bytes(&self) -> u64 {
        let initial = self.initial();
        let cnt = self.usable_count();
        let map = self.map.as_slice();
        let mut res = 0;
        for x in map[0..cnt].iter() {
            if *x != initial {
                res += 1;
            }
        }
        res
    }

    fn usable_count(&self) -> usize {
        self.map.as_slice().len()
    }

    fn hash(&self) -> u64 {

        let slice = &self.map.as_slice();

        let mut hasher = AHasher::new_with_keys(0, 0);
        let ptr = slice.as_ptr() as *const u8;
        let map_size = slice.len() / core::mem::size_of::<u32>();
        unsafe {
            hasher.write(from_raw_parts(ptr, map_size));
        }
        hasher.finish()
    }

    /// Reset the map
    #[inline]
    fn reset_map(&mut self) -> Result<(), Error> {
        // Normal memset, see https://rust.godbolt.org/z/Trs5hv
        let initial = self.initial();
        let cnt = self.usable_count();
        let map = self.map.as_mut_slice();
        for x in map[0..cnt].iter_mut() {
            *x = initial;
        }
        Ok(())
    }

    fn to_vec(&self) -> Vec<u32> {
        self.map.as_slice().to_vec()
    }

    /// Get the number of set entries with the specified indexes
    fn how_many_set(&self, indexes: &[usize]) -> usize {
        let initial = self.initial();
        let cnt = self.usable_count();
        let map = self.map.as_slice();
        let mut res = 0;
        for i in indexes {
            if *i < cnt && map[*i] != initial {
                res += 1;
            }
        }
        res
    }
}

impl<S> Observer<S> for VerdiXMLMapObserver 
where
    S: UsesInput,
{
    fn pre_exec(&mut self, _state: &mut S, _input: &S::Input) -> Result<(), Error> {
        //let initial = self.initial(); 
        self.map.clear();
        //let map = self.map.as_mut_slice();
        //for x in map.iter_mut() {
        //    *x = initial;
        //}
        Ok(())
    }

    #[inline]
    fn post_exec(
       &mut self,
        _state: &mut S,
        _input: &S::Input,
        _exit_kind: &ExitKind,
    ) -> Result<(), Error> {

        // Path to the gzip-compressed XML file
        let xml_file = match self.metric {
            VerdiCoverageMetric::Toggle => "tgl.verilog.data.xml",
            VerdiCoverageMetric::Line => "line.verilog.data.xml",
            VerdiCoverageMetric::FSM => "fsm.verilog.data.xml",
            VerdiCoverageMetric::Branch => "branch.verilog.data.xml",
            VerdiCoverageMetric::Condition => "cond.verilog.data.xml",
            VerdiCoverageMetric::Assert => "assert.verilog.data.xml",
        };

        let xml_file = format!("{}/{}/snps/coverage/db/testdata/test/{}", self.workdir, self.vdb, xml_file);
        let xml_file_ = xml_file.clone();

        // Open the gzip-compressed file
        let mut coverage_file = File::open(xml_file).expect("Unable to open file xml coverage file");
        
        let mut buffer = Vec::new();
        coverage_file.read_to_end(&mut buffer).expect("Unable to read the xml cov file tail the end");

        let mut gz = GzDecoder::new(&buffer[..]);
        let mut xml_str = String::new();
        gz.read_to_string(&mut xml_str).expect("Unable to unzip xml cove file using GzDecoder");

        let mut concatenated_bits = String::new();

        let mut coverable: u32 = 0;
        let mut covered: u32 = 0;

        let cursor = Cursor::new(xml_str);

        for line in cursor.lines() {
            if let Ok(line) = line {
                if line.trim_start().starts_with("<instance_data") {
                    // Find the start and end of the name attribute
                    if let Some(name_start) = line.find(r#"name=""#) {
                        let name_start = name_start + r#"name=""#.len();
                        if let Some(name_end) = line[name_start..].find('"') {
                            let name = &line[name_start..name_start + name_end];
                    
                            // Find the start and end of the value attribute
                            if let Some(value_start) = line.find(r#"value=""#) {
                                let value_start = value_start + r#"value=""#.len();
                                if let Some(value_end) = line[value_start..].find('"') {
                                    let value = &line[value_start..value_start + value_end];
                                    coverable += value.len() as u32;

                                    concatenated_bits.push_str(value);
                                }
                            }
                        }
                    }
                }
            }
        }

        // we saved the bitmap in str format into concatenated_bits
        // This consumes a bit of memory, but then it is easier to translate the String into a
        // concatenated chain of bit: bitmap.

        let bit_len = concatenated_bits.len();
        let mut start = 0;

        self.map.push(coverable);

        while start < bit_len {
            
            // process 32bits chunks at max
            let end = (start + 32).min(bit_len);
            let bit_chunk = &concatenated_bits[start..end];

            // translates str to u32 with radix 2
            let value = u32::from_str_radix(bit_chunk, 2).unwrap();

            // Normally, only last chunk might be not 32bits aligned so we add padding with 0
            let value = if end - start < 32 {
                value << (32 - (end - start))
            } else {
                value
            };

            covered += value.count_ones();

            self.map.push(value);
            start += 32;
        }
        self.map.insert(0, covered);

        Ok(())
    }
}

impl<'it> AsIter<'it> for VerdiXMLMapObserver
{
    type Item = u32;
    type IntoIter = Iter<'it, u32>;

    fn as_iter(&'it self) -> Self::IntoIter {
        let cnt = self.usable_count();
        self.map.as_slice()[..cnt].iter()
    }
}

impl<'it> AsIterMut<'it> for VerdiXMLMapObserver
{
    type Item = u32;
    type IntoIter = IterMut<'it, u32>;

    fn as_iter_mut(&'it mut self) -> Self::IntoIter {
        let cnt = self.usable_count();
        self.map.as_mut_slice()[..cnt].iter_mut()
    }
}

impl<OTA, OTB, S> DifferentialObserver<OTA, OTB, S> for VerdiXMLMapObserver
where
    OTA: ObserversTuple<S>,
    OTB: ObserversTuple<S>,
    S: UsesInput,
{
    fn pre_observe_first(&mut self, _observers: &mut OTA) -> Result<(), Error> {
        Ok(())
    }

    fn post_observe_first(&mut self, _observers: &mut OTA) -> Result<(), Error> {
        Ok(())
    }

    fn pre_observe_second(&mut self, _observers: &mut OTB) -> Result<(), Error> {
        Ok(())
    }
    
    fn post_observe_second(&mut self, _observers: &mut OTB) -> Result<(), Error> {
        Ok(())
    }
}


// TODO: Re-enable this test using vdb from open source design
#[cfg(feature = "std")]
#[cfg(test)]
mod tests {

    extern crate fs_extra;
    use libc::{c_uint, c_char, c_void};
    use nix::{sys::wait::waitpid,unistd::{fork, ForkResult}};
    use std::process;
    use libafl_bolts::prelude::StdRand;
    use libafl::prelude::BytesInput;
    use libafl::executors::{ExitKind};
    use libafl_bolts::current_time;
    use libafl::prelude::InMemoryCorpus;
    use libafl::prelude::Testcase;
    use libafl::prelude::ConstFeedback;
    use crate::verdi_xml_observer::VerdiXMLMapObserver;
    use crate::verdi_xml_observer::VerdiCoverageMetric;
    use libafl::prelude::StdState;
    use libafl::state::HasMaxSize;
    use libafl::observers::Observer;

    #[test]
    fn test_verdi_xml_observer() {

        let input = BytesInput::new(vec![1, 2, 3, 4]);

        let rand = StdRand::with_seed(current_time().as_nanos() as u64);
        let corpus = InMemoryCorpus::<BytesInput>::new();

        let mut feedback = ConstFeedback::new(true);
        let mut objective = ConstFeedback::new(false);


        let mut verdi_observer = VerdiXMLMapObserver::new(
                "verdi_map",
                &String::from("test.vdb"),
                ".",
                VerdiCoverageMetric::Toggle,
                &"chiptop0".to_string()
        );

        let mut state = StdState::new(
            rand,
            corpus,
            InMemoryCorpus::<BytesInput>::new(),
            &mut feedback,
            &mut objective,
        )
        .unwrap();
        state.set_max_size(1024);

        let _ = verdi_observer.post_exec(&mut state, &input, &ExitKind::Ok);
    }
}


// Copyright 2020 Jeremy Wall
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Common windows performance counter Paths

// CPU Metrics
pub const CPU_TOTAL_PCT: &'static str = r"\Processor Information(_Total)\% Processor Time";
pub const CPU_IDLE_PCT: &'static str = r"\Processor Information(_Total)\% Idle Time";
pub const CPU_USER_PCT: &'static str = r"\Processor Information(_Total)\% User Time";
pub const CPU_PRIVILEGED_PCT: &'static str = r"\Processor Information(_Total)\% Privileged Time";
pub const CPU_PRIORITY_PCT: &'static str = r"\Processor Information(_Total)\% Priority Time";
pub const CPU_FREQUENCY: &'static str = r"\Processor Information(_Total)\Processor Frequency";

// Memory metrics
pub const MEM_AVAILABLE_BYTES: &'static str = r"\Memory\Available Bytes";
pub const MEM_CACHE_BYTES: &'static str = r"\Memory\Cache Bytes";
pub const MEM_COMMITTED_BYTES: &'static str = r"\Memory\Committed Bytes";

// Network statistics
pub const NET_IFC_BYTES_RCVD_SEC: &'static str = r"\Network Interface(*)\Bytes Received/sec";
pub const NET_IFC_BYTES_SENT_SEC: &'static str = r"\Network Interface(*)\Bytes Sent/sec";
pub const NET_IFC_PKTS_RCVD_ERR: &'static str = r"\Network Interface(*)\Packets Received Errors"; // Count
pub const NET_IFC_PKTS_RCVD_DISCARD: &'static str =
    r"\Network Interface(*)\Packets Received Discarded"; // Count
pub const NET_IFC_PKTS_RCVD_SEC: &'static str = r"\Network Interface(*)\Packets Received/sec";
pub const NET_IFC_PKTS_SENT_SEC: &'static str = r"\Network Interface(*)\Packets Sent/sec";

// Disk statistics
pub const DISK_PCT_READ_TIME: &'static str = r"\PhysicalDisk(_Total)\% Disk Read Time";
pub const DISK_PCT_WRITE_TIME: &'static str = r"\PhysicalDisk(_Total)\% Disk Write Time";
pub const DISK_READ_BYTES_SEC: &'static str = r"\PhysicalDisk(_Total)\Disk Read Bytes/sec";
pub const DISK_WRITE_BYTES_SEC: &'static str = r"\PhysicalDisk(_Total)\Disk Write Bytes/sec";

// System statistics
pub const SYS_PROCESSES_COUNT: &'static str = r"\System\Processes"; // Count
pub const SYS_THREADS_COUNT: &'static str = r"\System\Threads"; // Count
pub const SYS_CONTEXT_SWITCH_SEC: &'static str = r"\System\Context Switches/sec";
pub const SYS_SYSTEM_CALLS_SEC: &'static str = r"\System\System Calls/sec";

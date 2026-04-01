import re
import sys

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

bootstrap_start = -1
bootstrap_end = -1
for i, line in enumerate(lines):
    if "pub fn bootstrap_stdlib" in line:
        bootstrap_start = i
    if "pub fn execute(" in line:
        bootstrap_end = i
        break

imports = """use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use duke_bytecode::instruction::ArrayType;
use duke_gc::HeapObject;
use duke_runtime::{Slot, VmError, VmResult};

use crate::context::{ClassContext, FieldEntry};
use crate::registry::{ClassRegistry, NativeControl};
use crate::threading::wait_for_all_java_threads;
use crate::{internal_name_to_binary_name, binary_name_to_internal_name};
use duke_loader::ClassLoader;

"""

with open("crates/duke-interpreter/src/natives.rs", "w") as f:
    f.write(imports)
    f.writelines(lines[bootstrap_start - 10:bootstrap_end])

import re
import sys

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

bootstrap_start = 41
bootstrap_end = 5339

# Copy the exact imports needed for natives and stdlib bootstrapping into `crates/duke-interpreter/src/natives.rs`

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
use crate::registry::ClassRegistry;

"""

with open("crates/duke-interpreter/src/natives.rs", "w") as f:
    f.write(imports)
    f.writelines(lines[bootstrap_start - 10:bootstrap_end])

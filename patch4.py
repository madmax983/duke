import re
import sys

with open("crates/duke-interpreter/src/natives.rs", "r") as f:
    lines = f.readlines()

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
use crate::{NativeControl, ClassLoader, wait_for_all_java_threads};

"""

with open("crates/duke-interpreter/src/natives.rs", "w") as f:
    f.write(imports)
    # The previous script had a bug where it re-wrote lines over the imports
    # Lets just re-read the original file again
    pass

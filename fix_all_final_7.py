with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    data = f.read()

data = data.replace('(class_name, method_name, pc)', '(`class_name`, `method_name`, `pc`)')
data = data.replace('(allocating_class, allocating_method, pc)', '(`allocating_class`, `allocating_method`, `pc`)')
data = data.replace('(class_name, method_name, handler_pc)', '(`class_name`, `method_name`, `handler_pc`)')
data = data.replace('(caller_class, cp_idx)', '(`caller_class`, `cp_idx`)')
data = data.replace('(class_name, method_name)', '(`class_name`, `method_name`)')

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(data)

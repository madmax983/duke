# 🔭 Vantage: Spec for Container Awareness (cgroups)

## 👤 User Story
"As a DevOps Engineer deploying Duke in a Kubernetes environment, I want the VM to be aware of container resource limits (cgroups v2), so that it doesn't over-allocate memory or spawn too many threads, which would lead to OOMKilled crashes or CPU throttling."

## ❓ The "So What?"
What business problem does this solve?
Modern enterprise workloads run almost exclusively in containers (Docker, Kubernetes). If a VM cannot read its cgroup constraints, it assumes it has access to the full host machine's resources. This causes the VM to allocate massive heaps or spawn thread pools that exceed the container's hard limits, resulting in the orchestration system immediately killing the process (OOMKill). Without container awareness, Duke cannot be reliably deployed in modern cloud-native environments, rendering it useless for cloud-first enterprise customers.

## 📈 Metric Definition
Success = A user can run Duke in a container with a strict memory limit (e.g., `docker run -m 512m`), and Duke automatically sizes its maximum heap to a safe percentage of that limit (e.g., 25-50%) rather than reading the physical host's RAM, and does not get OOMKilled during a heavy allocation workload.

## 🔍 Gap Analysis
- **Current State:** Duke likely reads system resources from standard OS APIs (e.g., `sysconf` or `sysinfo`), which report the host node's total physical memory and CPU cores, completely ignoring Docker/K8s cgroup limits.
- **Market/Standard Lib:** Standard JVMs introduced `+UseContainerSupport` (enabled by default since Java 10) to automatically parse `/sys/fs/cgroup/...` and adjust `MaxHeapSize` and `ActiveProcessorCount` accordingly.
- **The Gap:** Duke needs an internal mechanism to detect if it is running inside a container, parse the cgroup limits (especially cgroups v2, the modern standard), and use those values as the "physical" limits when bootstrapping the GC heap and thread pools.

## ✅ Acceptance Criteria
- Must read and respect cgroups v2 memory limits (`/sys/fs/cgroup/memory.max`).
- Must read and respect cgroups v2 CPU quotas (`/sys/fs/cgroup/cpu.max`).
- Must automatically scale default heap sizing based on container limits rather than host physical memory when containerized.
- Must fallback gracefully to host limits if cgroups are unavailable or not configured.
- Must expose the effective container limits to standard internal APIs (like `Runtime.getRuntime().availableProcessors()`).

## 🚫 Out of Scope
- Support for legacy cgroups v1 (focus entirely on v2 to start).
- Dynamic reconfiguration (if limits change while the VM is running, Duke does not need to react instantly).
- Interacting with K8s Downward API (purely rely on Linux cgroup filesystem).

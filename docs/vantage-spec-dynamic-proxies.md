# 🔭 Vantage: Spec for java.lang.reflect.Proxy (Dynamic Proxies)

## 👤 User Story
"As a Java Framework Developer using Duke, I want the VM to support `java.lang.reflect.Proxy`, so that I can dynamically generate classes at runtime that implement multiple interfaces and route method invocations through a single `InvocationHandler`."

## ❓ The "So What?"
What business problem does this solve?
Dynamic Proxies are the foundational building block for modern Enterprise Java. They are used extensively by Spring (for AOP, Transactions, and Repositories), Hibernate (for lazy-loading collections and entities), and Mockito (for creating test mocks). Without the ability to synthesize proxy classes at runtime, these frameworks completely fail to initialize. Implementing Dynamic Proxies transforms Duke from a static execution engine into a dynamic runtime capable of supporting complex middleware and modern development paradigms.

## 📈 Metric Definition
Success = A user can call `Proxy.newProxyInstance(loader, new Class<?>[]{MyInterface.class}, handler)`, cast the returned object to `MyInterface`, invoke a method on it, and successfully intercept the call inside their custom `InvocationHandler.invoke` method, receiving the correct method arguments.

## 🔍 Gap Analysis
- **Current State:** Duke can instantiate statically defined classes loaded from bytecode, but has no mechanism to dynamically synthesize new class definitions or map method invocations to a generic handler.
- **Market/Standard Lib:** Standard JVMs dynamically generate a byte array representing the `.class` file for the proxy and inject it directly into the classloader, usually under a synthesized name like `$Proxy0`.
- **The Gap:** We need a way to dynamically construct a class definition object in memory that implements the requested interfaces, define it in the class registry, and generate special bytecode (or native method intercepts) that delegate calls to the proxy instance's `InvocationHandler`.

## ✅ Acceptance Criteria
- Must implement the native bridge used by standard Java libraries to generate proxies.
- Must support synthesizing classes that implement one or more interfaces.
- The synthesized proxy must correctly map all methods defined in the interfaces to the `InvocationHandler.invoke` method.
- Must correctly package the arguments (primitives must be boxed) when passing them to `invoke`.
- Must unbox and type-check the return value from `invoke` before returning it to the caller.
- The synthesized proxy must route `hashCode`, `equals`, and `toString` to the `InvocationHandler` per the Java specification.

## 🚫 Out of Scope
- Bytecode generation libraries (like CGLib or ByteBuddy) modifying *existing* classes. This spec only covers interface-based proxies via `java.lang.reflect.Proxy`.
- Proxying standard Java classes (only interfaces are permitted by the specification).

# Payments engine - Design Notes

Ensuring correctness
- No Floating-point types. Only strictly precise integer value for precision guarantees.
- Extensive unit test suite. Aprox. 50 unit tests test for correct behaviour testing both happy path and all unhappy path variations.
- Integration test test for correct output formating, as well as proper input validation as well as correct end states.

Safety and Robustness
- Limiting unbounded Memory growth when more memory is not further reservable.
- Custom Error values with Error propagation.

Efficiency
- Sample Data from CSV file is read by sequentially using a Reader wrapped by the csv library.
- Currently does not support concurrent transaction processing, but is extendable by wrapping the engine struct internals in Arc + RwLocks/Mutexes.
- Valid Client ids are u16 and Tx ids even u32. This represents a considerable memory problem. Using try_reserve runtime panics are prevented but transactions might be rejected when memory  resources are limited. (More down below)

Maintainability
- Input, Output and processing are modularized enabling maintainability and exchangability.
    Want to read from TCP streams instead of a singular CSV file?
    Exchange the Input Code.
    Want to write to Disk instead of stdout?
    Exchange the Output code.

Numeric Representation & Precision
- Floating-point types (f32, f64) are not used, instead scaled i128 integers with a fixed scale of 10 ^ 4 are used.
    - Maximum positive value: (2 ^ 127 - 1) / 10 ^ 4
    - Maximum negative value: - 2 ^ 127 / 10 ^ 4
- This range is sufficient for any reasonable real world transaction volume.


Parsing & Normalization
- Amounts are fractional values with up to 4 decimal places.
- Accepted formats:
    - "x", "x.x", "x.xx", "x.xxx", "x.xxxx"
- Whole number strings are accepted up to the defined overflow boundries.
- Output formatting is normalized to always include a decimal point and at least 1 decimal digit.

Client & Transaction Semantics
- A client has 'available' & 'held' balances.
- A deposit increases the available funds. (If not overflow available)
- A withdrawal decreases the available funds. (If enough funds in available)
- A deposit moves funds from available to held. (If not overflow held and enough funds in available)
- A resolve moves funds back from held to available. (If not overflow available, and enough funds in held)
- A chargeback moves removes funds from held. (If enough funds in held) Also blocks the Client account, any further transactions will be rejected.
- Only deposits cann be disputed.
- Only disputed depoists can be resolved.
- Only disputed depoists can be chargedback.
- Accounts are never allowed to go into negative balance

Development Notes
- The in memory store for past transaction grow unbounded with every new deposit. (using try_reserve to prevent runtime panics, but will reject transactions if memory is too restricted)
- Since only deposits are dipsutable (and resolve/chargeback) no other transactions need to be stored.
- It could be argued that chargebacked deposits could be deleted as they cannot be disputed anymore.
- In a more elaborate system one might look at other complexity improvements like splitting the stored transactions differently. E.g:
    - One idea is to group them by timestamp and either write them to disk at some point or delete them meaning after x amt of time any transaction is indisputable.
    - Another idea is to store only the last x amount of transaction of any given user. This would mean only the past x amt of transaction of any user are disputable.

AI Usage
- Assisted with:
    - Implementing a serde serializer/deserialzer for fixed precision decimal values
    - documentation summarization
- Developer tooling:
    - RustAnalyzer LSP for autocompletion and function documentation templates
- Testing
    - some integration test test files

Roadmap
Possible next steps for scalability include:
- dependency injecting for different kinds of storage stores of past transactions/account balances to battle unbounded memory growth.
    Various Databases could be chosen to offload memory usage to use disk space instead as RAM is limited, Disk space is much cheaper and plentiful (but slower!).
- Weighing pro's and cons wrapping the Storage stores (currently in memory HashMaps) with Arcs + Mutexes/RWLock's to enable concurrent processing.
    Current implementation would loose efficiency by having to lock/unlock Mutexes/RwLocks when only a single File read is supported.

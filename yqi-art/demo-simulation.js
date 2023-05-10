import { CodeSimulator, Position, exit, display } from './fusion-blossom.js'

let d = 9
let code = await CodeSimulator.create(d)


display(code)

code.set_qubit_error(new Position(3, 2), "Z")
code.set_qubit_error(new Position(1, 0), "X")
code.set_qubit_error(new Position(5, 5), "Y")
await code.simulate()

display(code)

code.clear()  // call this function to clear all qubit errors
await code.simulate()

display(code)



exit()  // must be called, otherwise javascript never ends

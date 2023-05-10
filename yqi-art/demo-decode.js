import { CodeSimulator, Position, exit, display, animate_decoding } from './fusion-blossom.js'

let d = 9
let code = await CodeSimulator.create(d)


display(code)

// error chain 1
code.set_qubit_error(new Position(2, 1), "X")
code.set_qubit_error(new Position(2, 2), "X")
code.set_qubit_error(new Position(2, 3), "X")

// error chain 2
code.set_qubit_error(new Position(5, 5), "Y")
code.set_qubit_error(new Position(6, 5), "X")

await code.simulate()

display(code)

// start decoding, show an animation
await code.decode()
await animate_decoding(code)


exit()  // must be called, otherwise javascript never ends

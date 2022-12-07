use crate::instruction::Instruction;

#[derive(Debug, Eq, PartialEq)]
pub enum Exception {
    Error(String),
    RequestingInput,
}

impl Exception {
    pub fn error<S: Into<String>>(message: S) -> Exception {
        Exception::Error(message.into())
    }

    pub fn result<T>(self) -> Result<T, Exception> {
        Err(self)
    }
}

pub type EngineResult = Result<(), Exception>;

#[derive(Debug, Eq, PartialEq)]
pub enum InstructionPointer {
    Start,
    End,
    Index(usize),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Engine {
    pub tape: Vec<u8>,
    pub tape_pointer: usize,
    pub instructions: Vec<Instruction>,
    pub instruction_pointer: InstructionPointer,
    pub history: Vec<Instruction>,
    pub output: Vec<u8>,
    pub input: Vec<u8>,
    pub input_cell_history: Vec<u8>,
}

impl Engine {
    pub fn new(instructions: Vec<Instruction>) -> Engine {
        Engine {
            tape: vec![0],
            tape_pointer: 0,
            instructions,
            instruction_pointer: InstructionPointer::Start,
            history: vec![],
            output: vec![],
            input: vec![],
            input_cell_history: vec![],
        }
    }

    pub fn load_instructions(&mut self, instructions: Vec<Instruction>) {
        self.instructions = instructions;
    }

    pub fn goto(&mut self, instruction_index: usize) -> EngineResult {
        if instruction_index < self.instructions.len() {
            self.instruction_pointer = InstructionPointer::Index(instruction_index);
            Ok(())
        } else {
            Exception::error(format!(
                "no instruction at position {} (max {})",
                instruction_index,
                self.instructions.len() - 1
            ))
            .result()
        }
    }

    pub fn step(&mut self) -> EngineResult {
        match self.current_instruction() {
            Some(instruction) => (instruction.exec)(self).map(|_| {
                self.history.push(instruction);
            }),
            None => self.next_instruction(),
        }
    }

    pub fn undo(&mut self) -> EngineResult {
        let instruction = self
            .history
            .last()
            .ok_or_else(|| Exception::error("no previous instruction to undo"))?;

        (instruction.unexec)(self).map(|_| {
            self.history.pop();
        })
    }

    pub fn current_instruction(&self) -> Option<Instruction> {
        match self.instruction_pointer {
            InstructionPointer::Start => None,
            InstructionPointer::End => None,
            InstructionPointer::Index(i) => Some(self.instructions[i]),
        }
    }

    pub fn next_instruction(&mut self) -> EngineResult {
        match self.instruction_pointer {
            InstructionPointer::End => {
                Exception::error("already at the end of the instruction list").result()
            }
            InstructionPointer::Start => {
                self.instruction_pointer = InstructionPointer::Index(0);
                Ok(())
            }
            InstructionPointer::Index(i) if i + 1 == self.instructions.len() => {
                self.instruction_pointer = InstructionPointer::End;
                Ok(())
            }
            InstructionPointer::Index(i) => {
                self.instruction_pointer = InstructionPointer::Index(i + 1);
                Ok(())
            }
        }
    }

    pub fn prev_instruction(&mut self) -> EngineResult {
        match self.instruction_pointer {
            InstructionPointer::Start => {
                Exception::error("already at the start of the instruction list").result()
            }
            InstructionPointer::End => {
                self.instruction_pointer = InstructionPointer::Index(self.instructions.len() - 1);
                Ok(())
            }
            InstructionPointer::Index(i) if i == 0 => {
                self.instruction_pointer = InstructionPointer::Start;
                Ok(())
            }
            InstructionPointer::Index(i) => {
                self.instruction_pointer = InstructionPointer::Index(i - 1);
                Ok(())
            }
        }
    }

    pub fn goto_next(&mut self, goto: Instruction) -> EngineResult {
        let start = match self.instruction_pointer {
            InstructionPointer::End => {
                Exception::error("already at the end of the instruction list").result()
            }
            InstructionPointer::Start => Ok(0),
            InstructionPointer::Index(i) => Ok(i + 1),
        }?;

        let rest = self.instructions.iter().skip(start);
        for (i, instruction) in rest.enumerate() {
            if instruction == &goto {
                self.instruction_pointer = InstructionPointer::Index(start + i);
                return Ok(());
            }
        }

        Exception::error(format!("no next {} instruction found", goto.symbol)).result()
    }

    pub fn goto_prev(&mut self, goto: Instruction) -> EngineResult {
        let end = match self.instruction_pointer {
            InstructionPointer::Start => {
                Exception::error("already at the start of the instruction list").result()
            }
            InstructionPointer::End => Ok(self.instructions.len() - 1),
            InstructionPointer::Index(i) => Ok(i),
        }?;

        let rest = self.instructions.iter().take(end);
        for (i, instruction) in rest.rev().enumerate() {
            if instruction == &goto {
                self.instruction_pointer = InstructionPointer::Index(end - i - 1);
                return Ok(());
            }
        }

        Exception::error(format!("no previous {} instruction found", goto.symbol)).result()
    }

    pub fn next_cell(&mut self) -> EngineResult {
        self.tape_pointer += 1;
        // expand the tape if the cell is new
        if self.tape_pointer == self.tape.len() {
            self.tape.push(0);
        }

        Ok(())
    }

    pub fn prev_cell(&mut self) -> EngineResult {
        self.tape_pointer -= 1;

        Ok(())
    }

    pub fn cell(&self) -> u8 {
        self.tape[self.tape_pointer]
    }

    pub fn set_cell(&mut self, value: u8) {
        self.tape[self.tape_pointer] = value;
    }

    pub fn map_cell(&mut self, f: fn(u8) -> u8) {
        let value = self.cell();
        self.set_cell(f(value));
    }

    pub fn input(&mut self, buffered: &mut Vec<u8>) {
        let mut input = vec![];
        input.append(buffered);
        input.append(&mut self.input);
        self.input = input;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOOP_A: Instruction = Instruction {
        symbol: 'a',
        exec: |_| Ok(()),
        unexec: |_| Ok(()),
    };
    const NOOP_B: Instruction = Instruction {
        symbol: 'b',
        exec: |_| Ok(()),
        unexec: |_| Ok(()),
    };
    const NOOP_C: Instruction = Instruction {
        symbol: 'c',
        exec: |_| Ok(()),
        unexec: |_| Ok(()),
    };

    fn ok(result: EngineResult) {
        assert_eq!(result, Ok(()))
    }

    #[test]
    fn new_builds_blank_program() {
        let program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C]);

        assert_eq!(
            program,
            Engine {
                tape: vec![0],
                tape_pointer: 0,
                instructions: vec![NOOP_A, NOOP_B, NOOP_C],
                instruction_pointer: InstructionPointer::Start,
                history: vec![],
                output: vec![],
                input: vec![],
                input_cell_history: vec![],
            }
        );
    }

    #[test]
    fn goto_sets_instruction_pointer() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C]);

        ok(program.goto(1));

        assert_eq!(program.current_instruction(), Some(NOOP_B));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(1));
    }

    #[test]
    fn goto_overrun_fails_gracefully() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C]);

        assert!(program.goto(3).is_err());
        assert_eq!(program.instruction_pointer, InstructionPointer::Start);
    }

    #[test]
    fn goto_next_moves_to_next_instruction() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C, NOOP_B, NOOP_A, NOOP_C]);

        ok(program.goto_next(NOOP_C));

        assert_eq!(program.current_instruction(), Some(NOOP_C));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(2));
    }

    #[test]
    fn goto_next_twice_moves_to_second_instruction() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C, NOOP_B, NOOP_A, NOOP_C]);

        ok(program.goto_next(NOOP_C));
        ok(program.goto_next(NOOP_C));

        assert_eq!(program.current_instruction(), Some(NOOP_C));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(5));
    }

    #[test]
    fn goto_next_fails_gracefully_on_overrun() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C, NOOP_A]);

        ok(program.goto_next(NOOP_C));

        assert!(program.goto_next(NOOP_B).is_err());
        assert_eq!(program.current_instruction(), Some(NOOP_C));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(2));
    }

    #[test]
    fn goto_prev_moves_to_prev_instruction() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C, NOOP_B, NOOP_A, NOOP_C]);

        ok(program.goto(5));
        ok(program.goto_prev(NOOP_A));

        assert_eq!(program.current_instruction(), Some(NOOP_A));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(4));
    }

    #[test]
    fn goto_prev_twice_moves_to_second_instruction() {
        let mut program = Engine::new(vec![NOOP_A, NOOP_B, NOOP_C, NOOP_B, NOOP_A, NOOP_C]);

        ok(program.goto(5));
        ok(program.goto_prev(NOOP_A));
        ok(program.goto_prev(NOOP_A));

        assert_eq!(program.current_instruction(), Some(NOOP_A));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(0));
    }

    #[test]
    fn goto_prev_fails_gracefully_on_underrun() {
        let mut program = Engine::new(vec![NOOP_C, NOOP_A, NOOP_B, NOOP_C]);

        ok(program.goto(3));
        ok(program.goto_prev(NOOP_A));

        assert!(program.goto_prev(NOOP_B).is_err());
        assert_eq!(program.current_instruction(), Some(NOOP_A));
        assert_eq!(program.instruction_pointer, InstructionPointer::Index(1));
    }
}

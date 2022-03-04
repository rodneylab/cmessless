pub struct Stack<T> {
    structure: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack {
            structure: Vec::new(),
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.structure.last()
    }

    pub fn pop(&mut self) -> Option<T> {
        self.structure.pop()
    }

    pub fn push(&mut self, element: T) {
        self.structure.push(element)
    }

    // fn len(&self) -> usize {
    //     self.structure.len()
    // }

    // fn is_empty(&self) -> bool {
    //     self.structure.is_empty()
    // }
}

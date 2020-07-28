use std::fmt::{self,Debug};
use std::sync::{Arc,Condvar,Mutex};

struct InnerOne<T> {
    payload: Mutex<Option<T>>,
    cond: Condvar,
}
impl<T> InnerOne<T> {
    fn new() -> InnerOne<T> {
        InnerOne {
            payload: Mutex::new(None),
            cond: Condvar::new(),
        }
    }
    fn set(&self, t: T) {
        let mut lock = self.payload.lock().unwrap();
        *lock = Some(t);
        self.cond.notify_one();
    }
    fn wait(&self) -> T {
        let mut lock = self.payload.lock().unwrap();
        while lock.is_none() {
            lock = self.cond.wait(lock).unwrap();
        }
        lock.take().unwrap()
    }
}

pub struct OneGet<T>(Arc<InnerOne<Option<T>>>);
impl<T> OneGet<T> {
    pub fn is_ready(&self) -> bool {
        // relaxed variant
        Arc::strong_count(&self.0) == 1
    }
    pub fn wait(self) -> Option<T> {
        self.0.wait()
    }
    pub fn try_get(self) -> Result<Option<T>,OneGet<T>> {
        match Arc::try_unwrap(self.0) {
            Ok(inner) => Ok(inner.wait()),
            Err(arc_inner) => Err(OneGet(arc_inner)),
        }
    }
}
impl<T> Debug for OneGet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "OneGet")
    }
}

pub struct OneSet<T>(Arc<InnerOne<Option<T>>>,bool);
impl<T> OneSet<T> {
    pub fn is_needed(&self) -> bool {
        // relaxed variant
        Arc::strong_count(&self.0) == 2
    }
    pub fn set(mut self, t: T) {
        self.0.set(Some(t));
        self.1 = true;
    }
}
impl<T> Debug for OneSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "OneSet")
    }
}   
impl<T> Drop for OneSet<T> {
    fn drop(&mut self) {
        if !self.1 {
            self.0.set(None);
        }
    }
}

pub fn oneshot<T>() -> (OneSet<T>,OneGet<T>) {
    let r = Arc::new(InnerOne::new());
    (OneSet(r.clone(),false),OneGet(r))
}


#[cfg(test)]
mod tests {
    use super::oneshot;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_wait_setting() {
        let (tx,rx) = oneshot();
        let h = thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            tx.set(5);
        });
        assert_eq!(rx.wait(),Some(5));
        h.join().unwrap();
    }
    
    #[test]
    fn test_wait_getting() {
        let (tx,rx) = oneshot();
        let h = thread::spawn(move || {                
            tx.set(3);
        });
        thread::sleep(Duration::from_millis(500));
        assert_eq!(rx.wait(),Some(3));
        h.join().unwrap();
    }
    
    #[test]
    fn test_drop_setter() {
        let (tx,rx) = oneshot::<u64>();
        let h = thread::spawn(move || {                
            let _tx = tx;
        });
        thread::sleep(Duration::from_millis(500));
        assert_eq!(rx.wait(),None);
        h.join().unwrap();
    }
    
}


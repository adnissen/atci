use atci_rust::FuzzyFilterableVec;

#[test]
fn test_basic_vec_functionality() {
    let mut fvec = FuzzyFilterableVec::new();
    
    fvec.push("hello".to_string());
    fvec.push("world".to_string());
    fvec.push("rust".to_string());
    
    assert_eq!(fvec.len(), 3);
    assert_eq!(fvec[0], "hello");
    assert_eq!(fvec[1], "world");
    assert_eq!(fvec[2], "rust");
}

#[test]
fn test_filter_by_string() {
    let mut fvec = FuzzyFilterableVec::new();
    
    fvec.push("Hello World".to_string());
    fvec.push("Goodbye Mars".to_string());
    fvec.push("HELLO Universe".to_string());
    fvec.push("rust programming".to_string());
    fvec.push("Python coding".to_string());
    
    let results = fvec.filter_by_string("hello");
    assert_eq!(results.len(), 2);
    assert_eq!(*results[0], "Hello World");
    assert_eq!(*results[1], "HELLO Universe");
    
    let rust_results = fvec.filter_by_string("RUST");
    assert_eq!(rust_results.len(), 1);
    assert_eq!(*rust_results[0], "rust programming");
    
    let empty_results = fvec.filter_by_string("nonexistent");
    assert_eq!(empty_results.len(), 0);
}

#[test]
fn test_case_insensitive_search() {
    let fvec: FuzzyFilterableVec<String> = vec![
        "CamelCase".to_string(),
        "snake_case".to_string(),
        "SCREAMING_SNAKE_CASE".to_string(),
        "kebab-case".to_string(),
    ].into();
    
    let results = fvec.filter_by_string("case");
    assert_eq!(results.len(), 4);
    
    let camel_results = fvec.filter_by_string("CAMEL");
    assert_eq!(camel_results.len(), 1);
    assert_eq!(*camel_results[0], "CamelCase");
}

#[test]
fn test_partial_string_matching() {
    let fvec: FuzzyFilterableVec<String> = vec![
        "The quick brown fox".to_string(),
        "jumps over the lazy dog".to_string(),
        "A quick solution".to_string(),
        "Slow and steady".to_string(),
    ].into();
    
    let quick_results = fvec.filter_by_string("quick");
    assert_eq!(quick_results.len(), 2);
    
    let the_results = fvec.filter_by_string("the");
    assert_eq!(the_results.len(), 2);
}

#[test]
fn test_from_vec_conversion() {
    let regular_vec = vec!["item1".to_string(), "item2".to_string(), "item3".to_string()];
    let fvec = FuzzyFilterableVec::from_vec(regular_vec);
    
    assert_eq!(fvec.len(), 3);
    
    let results = fvec.filter_by_string("item");
    assert_eq!(results.len(), 3);
}

#[test]
fn test_into_vec_conversion() {
    let mut fvec = FuzzyFilterableVec::new();
    fvec.push("test1".to_string());
    fvec.push("test2".to_string());
    
    let regular_vec: Vec<String> = fvec.into();
    assert_eq!(regular_vec.len(), 2);
    assert_eq!(regular_vec[0], "test1");
    assert_eq!(regular_vec[1], "test2");
}

#[test]
fn test_iterator_functionality() {
    let fvec: FuzzyFilterableVec<i32> = vec![1, 2, 3, 4, 5].into();
    
    let sum: i32 = fvec.iter().sum();
    assert_eq!(sum, 15);
    
    let collected: Vec<i32> = fvec.into_iter().collect();
    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_deref_functionality() {
    let fvec: FuzzyFilterableVec<String> = vec!["a".to_string(), "b".to_string()].into();
    
    // Test that we can use Vec methods via Deref
    assert_eq!(fvec.first(), Some(&"a".to_string()));
    assert_eq!(fvec.last(), Some(&"b".to_string()));
    assert!(!fvec.is_empty());
}
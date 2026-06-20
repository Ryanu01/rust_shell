use super::Step;

fn initial_step(arr: &[u32]) -> Step {
    Step {
        array: arr.to_vec(),
        compare: None,
        swap: None,
        sorted: vec![false; arr.len()],
        label: "Initial array".to_string(),
    }
}

fn sorted_step(arr: &[u32]) -> Step {
    Step {
        array: arr.to_vec(),
        compare: None,
        swap: None,
        sorted: vec![true; arr.len()],
        label: "Array is fully sorted!".to_string(),
    }
}

pub fn bubble_sort_steps(arr: &[u32]) -> Vec<Step> {
    let mut a = arr.to_vec();
    let n = a.len();
    let mut steps = Vec::new();
    steps.push(initial_step(arr));

    for i in 0..n {
        let mut swapped = false;
        for j in 0..n - 1 - i {
            steps.push(Step {
                array: a.clone(),
                compare: Some((j, j + 1)),
                swap: None,
                sorted: (n - i..n).map(|k| k < n).collect(),
                label: format!("Comparing {} and {}", a[j], a[j + 1]),
            });
            if a[j] > a[j + 1] {
                a.swap(j, j + 1);
                swapped = true;
                steps.push(Step {
                    array: a.clone(),
                    compare: None,
                    swap: Some((j, j + 1)),
                    sorted: (n - i..n).map(|k| k < n).collect(),
                    label: format!("Swapped {} and {}", a[j + 1], a[j]),
                });
            }
        }
        if !swapped {
            break;
        }
    }

    steps.push(sorted_step(&a));
    steps
}

pub fn selection_sort_steps(arr: &[u32]) -> Vec<Step> {
    let mut a = arr.to_vec();
    let n = a.len();
    let mut steps = Vec::new();
    steps.push(initial_step(arr));

    for i in 0..n {
        let mut min_idx = i;
        for j in i + 1..n {
            steps.push(Step {
                array: a.clone(),
                compare: Some((min_idx, j)),
                swap: None,
                sorted: (0..i).map(|k| k < n).collect(),
                label: format!("Finding minimum: comparing {} and {}", a[min_idx], a[j]),
            });
            if a[j] < a[min_idx] {
                min_idx = j;
            }
        }
        if min_idx != i {
            a.swap(i, min_idx);
            steps.push(Step {
                array: a.clone(),
                compare: None,
                swap: Some((i, min_idx)),
                sorted: (0..=i).map(|k| k < n).collect(),
                label: format!("Swapped {} and {}", a[i], a[min_idx]),
            });
        } else {
            steps.push(Step {
                array: a.clone(),
                compare: None,
                swap: None,
                sorted: (0..=i).map(|k| k < n).collect(),
                label: format!("{} is already in position", a[i]),
            });
        }
    }

    steps.push(sorted_step(&a));
    steps
}

pub fn insertion_sort_steps(arr: &[u32]) -> Vec<Step> {
    let mut a = arr.to_vec();
    let n = a.len();
    let mut steps = Vec::new();
    steps.push(initial_step(arr));

    for i in 1..n {
        let key = a[i];
        let mut j = i;
        while j > 0 && a[j - 1] > key {
            steps.push(Step {
                array: a.clone(),
                compare: Some((j - 1, j)),
                swap: None,
                sorted: (0..i).map(|k| k < n).collect(),
                label: format!("Shifting {} right", a[j - 1]),
            });
            a[j] = a[j - 1];
            steps.push(Step {
                array: a.clone(),
                swap: Some((j - 1, j)),
                compare: None,
                sorted: (0..i).map(|k| k < n).collect(),
                label: format!("Moved {} to position {}", a[j], j),
            });
            j -= 1;
        }
        a[j] = key;
        steps.push(Step {
            array: a.clone(),
            compare: None,
            swap: None,
            sorted: (0..=i).map(|k| k < n).collect(),
            label: format!("Inserted {} at position {}", key, j),
        });
    }

    steps.push(sorted_step(&a));
    steps
}

pub fn merge_sort_steps(arr: &[u32]) -> Vec<Step> {
    let mut a = arr.to_vec();
    let mut steps = Vec::new();
    steps.push(initial_step(arr));
    let mut sorted = vec![false; a.len()];

    let mut width = 1;
    let n = a.len();
    while width < n {
        let mut left = 0;
        while left < n {
            let mid = left + width;
            if mid >= n {
                break;
            }
            let right = (left + 2 * width).min(n);
            let temp = a.clone();
            let mut i = left;
            let mut j = mid;
            let mut k = left;
            while i < mid && j < right {
                steps.push(Step {
                    array: temp.clone(),
                    compare: Some((i, j)),
                    swap: None,
                    sorted: sorted.clone(),
                    label: format!("Comparing {} and {}", temp[i], temp[j]),
                });
                if temp[i] <= temp[j] {
                    a[k] = temp[i];
                    i += 1;
                } else {
                    a[k] = temp[j];
                    j += 1;
                }
                k += 1;
            }
            while i < mid {
                a[k] = temp[i];
                i += 1;
                k += 1;
            }
            while j < right {
                a[k] = temp[j];
                j += 1;
                k += 1;
            }
            if left == 0 && right >= n {
                for idx in left..right {
                    sorted[idx] = true;
                }
            }
            steps.push(Step {
                array: a.clone(),
                compare: None,
                swap: None,
                sorted: sorted.clone(),
                label: format!("Merged segment [{}, {})", left, right),
            });
            left += 2 * width;
        }
        width *= 2;
    }

    steps.push(sorted_step(&a));
    steps
}

pub fn quick_sort_steps(arr: &[u32]) -> Vec<Step> {
    let mut a = arr.to_vec();
    let mut steps = Vec::new();
    steps.push(initial_step(arr));
    let mut sorted = vec![false; a.len()];

    fn partition(
        a: &mut Vec<u32>,
        low: usize,
        high: usize,
        steps: &mut Vec<Step>,
        sorted: &mut Vec<bool>,
    ) -> usize {
        let pivot = a[high];
        let mut i = low;
        for j in low..high {
            steps.push(Step {
                array: a.clone(),
                compare: Some((j, high)),
                swap: None,
                sorted: sorted.clone(),
                label: format!("Comparing {} with pivot {}", a[j], pivot),
            });
            if a[j] <= pivot {
                a.swap(i, j);
                if i != j {
                    steps.push(Step {
                        array: a.clone(),
                        swap: Some((i, j)),
                        compare: None,
                        sorted: sorted.clone(),
                        label: format!("Swapped {} and {}", a[i], a[j]),
                    });
                }
                i += 1;
            }
        }
        a.swap(i, high);
        if i != high {
            steps.push(Step {
                array: a.clone(),
                swap: Some((i, high)),
                compare: None,
                sorted: sorted.clone(),
                label: format!("Placed pivot {} at position {}", a[i], i),
            });
        }
        sorted[i] = true;
        i
    }

    fn qs(
        a: &mut Vec<u32>,
        low: usize,
        high: usize,
        steps: &mut Vec<Step>,
        sorted: &mut Vec<bool>,
    ) {
        if low < high {
            let p = partition(a, low, high, steps, sorted);
            if p > 0 {
                qs(a, low, p - 1, steps, sorted);
            }
            qs(a, p + 1, high, steps, sorted);
        } else if low == high && low < a.len() {
            sorted[low] = true;
        }
    }

    if !a.is_empty() {
        let last = a.len() - 1;
        qs(&mut a, 0, last, &mut steps, &mut sorted);
    }

    steps.push(sorted_step(&a));
    steps
}

pub fn heap_sort_steps(arr: &[u32]) -> Vec<Step> {
    let mut a = arr.to_vec();
    let n = a.len();
    let mut steps = Vec::new();
    steps.push(initial_step(arr));
    let mut sorted = vec![false; n];

    fn heapify(
        a: &mut Vec<u32>,
        n: usize,
        i: usize,
        steps: &mut Vec<Step>,
        sorted: &mut Vec<bool>,
    ) {
        let mut largest = i;
        let left = 2 * i + 1;
        let right = 2 * i + 2;

        if left < n {
            steps.push(Step {
                array: a.clone(),
                compare: Some((left, largest)),
                swap: None,
                sorted: sorted.clone(),
                label: format!("Comparing {} and {}", a[left], a[largest]),
            });
            if a[left] > a[largest] {
                largest = left;
            }
        }
        if right < n {
            steps.push(Step {
                array: a.clone(),
                compare: Some((right, largest)),
                swap: None,
                sorted: sorted.clone(),
                label: format!("Comparing {} and {}", a[right], a[largest]),
            });
            if a[right] > a[largest] {
                largest = right;
            }
        }
        if largest != i {
            a.swap(i, largest);
            steps.push(Step {
                array: a.clone(),
                swap: Some((i, largest)),
                compare: None,
                sorted: sorted.clone(),
                label: format!("Swapped {} and {} (heapify)", a[i], a[largest]),
            });
            heapify(a, n, largest, steps, sorted);
        }
    }

    for i in (0..n / 2).rev() {
        heapify(&mut a, n, i, &mut steps, &mut sorted);
    }

    for i in (1..n).rev() {
        a.swap(0, i);
        sorted[i] = true;
        steps.push(Step {
            array: a.clone(),
            swap: Some((0, i)),
            compare: None,
            sorted: sorted.clone(),
            label: format!("Extracted max {}, placed at end", a[i]),
        });
        heapify(&mut a, i, 0, &mut steps, &mut sorted);
    }
    sorted[0] = true;

    steps.push(sorted_step(&a));
    steps
}

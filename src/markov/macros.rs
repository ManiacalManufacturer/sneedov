macro_rules! get_occurrence {
    ($db:ident, $e:expr) => {{
        let result;
        let vec = $db.get_single_occurrences($e).await?;
        {
            let mut rng = thread_rng();
            result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
        }
        result
    }};
    ($db:ident, $e1:expr, $e2:expr) => {{
        let result: (u64, u64);
        let vec = $db.get_double_occurrences($e1, $e2).await?;
        {
            let mut rng = thread_rng();
            result = *vec.choose_weighted(&mut rng, |item| item.1)?;
        }
        result
    }};
    (reverse $db:ident, $e:expr) => {{
        let result;
        let vec = $db.get_prev_single_occurrences($e).await?;
        {
            let mut rng = thread_rng();
            result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
        }
        result
    }};
    (reverse $db:ident, $e1:expr, $e2:expr) => {{
        let result: (u64, u64);
        let vec = $db.get_prev_double_occurrences($e1, $e2).await?;
        {
            let mut rng = thread_rng();
            result = *vec.choose_weighted(&mut rng, |item| item.1)?;
        }
        result
    }};
}

macro_rules! generate {
    ($self:ident, $index:expr, $old_index:expr, $end:expr) => {{
        let mut index = $index;
        let mut index_result;
        let mut old_index = $old_index;
        let mut sentence = String::new();

        loop {
            (old_index, index_result) = (index, $self.next_word(old_index, index).await);
            match index_result {
                Ok(i) => {
                    index = i;
                }
                Err(_) => {
                    break;
                }
            }

            if index == $end {
                break;
            }

            let word = $self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if !sentence.is_empty() && !is_punc {
                sentence.push(' ');
            }
            sentence.push_str(&word);
        }

        sentence
    }};
    (reply $self:ident, $index:expr, $old_index:expr, $end:expr) => {{
        let mut index = $index;
        let mut index_result; //temp solution
        let mut old_index = $old_index;
        let mut sentence = String::new();

        loop {
            if index == $end {
                break;
            }

            let word = $self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if !sentence.is_empty() && !is_punc {
                sentence.push(' ');
            }

            sentence.push_str(&word);
            (old_index, index_result) = (index, $self.next_word(old_index, index).await);
            match index_result {
                Ok(i) => {
                    index = i;
                }
                Err(_) => {
                    break;
                }
            }
        }

        sentence
    }};
    (reverse $self:ident, $index:expr, $old_index:expr, $end:expr) => {{
        let mut index = $index;
        let mut index_result; //temp solution
                              //
        let mut old_index = $old_index;
        let mut sentence = String::new();

        let mut was_punc = false;

        loop {
            if index == $end {
                break;
            }

            let word = $self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if !sentence.is_empty() && !was_punc {
                sentence.insert(0, ' ');
            } else {
                was_punc = false;
            }

            if is_punc {
                was_punc = true;
            }

            sentence.insert_str(0, &word);
            (old_index, index_result) = (index, $self.prev_word(index, old_index).await);
            match index_result {
                Ok(i) => {
                    index = i;
                }
                Err(_) => {
                    break;
                }
            }
        }

        sentence
    }};
}

pub(crate) use generate;
pub(crate) use get_occurrence;

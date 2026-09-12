#[macro_export]
macro_rules! implement_defs_builder {
    (
        $ty:ty:
        $(
            $alloc_method:ident,
            $arena_field:ident,
            $id_type:ident,
            $data_type:ty
        ),* $(,)?
    ) => {
        impl $ty {
            $(
                #[inline]
                pub fn $alloc_method(&mut self, data: $data_type) -> $id_type {
                    let id = self.defs.$arena_field.alloc(data);
                    $id_type(id)
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! implement_defs_reader {
    (
        $ty:ty:
        $(
            $arena_field:ident,
            $values_method:ident,
            $id_type:ident,
            $data_type:ty
        ),* $(,)?
    ) => {
        impl $ty {
            $(
                pub fn $arena_field(
                    &self,
                ) -> impl ExactSizeIterator<Item = ($id_type, &$data_type)> + DoubleEndedIterator + Clone {
                    self.defs
                        .$arena_field
                        .iter()
                        .map(|(id, data)| ($id_type(id), data))
                }

                pub fn $values_method(
                    &self,
                ) -> impl ExactSizeIterator<Item = &$data_type> + DoubleEndedIterator + Clone {
                    self.defs
                        .$arena_field
                        .values()
                }
            )*
        }

        $(
            impl std::ops::Index<$id_type> for $ty {
                type Output = $data_type;

                #[inline]
                fn index(&self, index: $id_type) -> &Self::Output {
                    &self.defs.$arena_field[index.0]
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! implement_arenas_builder {
    (
        $ty:ty:
        $(
            $alloc_method:ident,
            $arena_field:ident,
            $id_type:ident,
            $data_type:ty
        ),* $(,)?
    ) => {
        impl $ty {
            $(
                #[inline]
                pub fn $alloc_method(&mut self, data: $data_type) -> $id_type {
                    self.$arena_field.alloc(data)
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! implement_arenas_reader {
    (
        $ty:ty:
        $(
            $arena_field:ident,
            $values_method:ident,
            $id_type:ident,
            $data_type:ty
        ),* $(,)?
    ) => {
        impl $ty {
            $(
                pub fn $arena_field(
                    &self,
                ) -> impl ExactSizeIterator<Item = ($id_type, &$data_type)> + DoubleEndedIterator + Clone {
                    self.$arena_field.iter()
                }

                pub fn $values_method(
                    &self,
                ) -> impl ExactSizeIterator<Item = &$data_type> + DoubleEndedIterator + Clone {
                    self.$arena_field.values()
                }
            )*
        }

        $(
            impl std::ops::Index<$id_type> for $ty {
                type Output = $data_type;

                #[inline]
                fn index(&self, index: $id_type) -> &Self::Output {
                    &self.$arena_field[index]
                }
            }
        )*
    };
}

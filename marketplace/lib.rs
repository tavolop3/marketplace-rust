#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod marketplace {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Marketplace {
        usuarios: Mapping<AccountId,Usuario>,                   // (id_usuario, datos_usuario)
        publicaciones: Mapping<AccountId, Vec<Producto>>,       // (id_vendedor, lista_de_productos)
        ordenes_compra: Mapping<AccountId, Vec<OrdenCompra>>    // (id_comprador, lista_de_ordenes)
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub enum ErrorSistema {
        UsuarioNoRegistrado
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub struct Usuario {
        username: String,
        rol: Rol,
        account_id: AccountId

        //Las calificaciones se veran mas adelante
        //calificacion_promedio: u8
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum Rol {
        Comprador,
        Vendedor,
        CompradorYVendedor,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub struct Producto {
        nombre_producto: String,
        descripcion: String,
        categoria: Categoria,
        precio: u64,
        stock: u64
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum Categoria {
        Computacion,
        Ropa,
        Herramienta,
        Mueble
    }
    
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub struct OrdenCompra {
        estado: Estado, 
        producto: Producto,
        comprador_id: AccountId,
        vendedor_id: AccountId,
        peticion_cancelacion: bool, // Solo el COMPRADOR puede hacer la peticion de cancelacion
        cacelacion_total: bool      // El VENDEDOR debe aprobar la peticion de cancelacion

        /* -- Se deja para mas adelante
        calificacion_al_comprador: u8,
        calificacion_al_vendedor: u8,
        */
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum Estado {
        Pendiente, 
        Enviada,
        Recibida, 
        Cancelada
    }

    /* -- Se deja para mas adelante
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum UnoACinco {
        Uno = 1,
        Dos = 2,
        Tres = 3,
        Cuatro = 4,
        Cinco = 5,
    }
    */

    impl Marketplace {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { 
                usuarios: Default::default(), 
                publicaciones: Default::default(),
                ordenes_compra: Default::default()
            }
        }

        // Funcion que registra un nuevo usuario en el sistema
        #[ink(message)]
        pub fn registrar_usuario(&mut self, username: String, rol: Rol) {
            self.usuarios.insert(Self::env().caller(),&Usuario {
                account_id: Self::env().caller(),
                username,
                rol
            });
        }

        // Funcion que busca un usuario en el sistema y lo retorna
        #[ink(message)]
        pub fn get_usuarios(&self) -> Result<Usuario,ErrorSistema> {
            self.usuarios.get(Self::env().caller()).ok_or(ErrorSistema::UsuarioNoRegistrado)
        }
        
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let marketplace = Marketplace::default();
            assert_eq!(marketplace.get(), false);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let mut marketplace = Marketplace::new(false);
            assert_eq!(marketplace.get(), false);
            marketplace.flip();
            assert_eq!(marketplace.get(), true);
        }
    }

    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::ContractsBackend;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let mut constructor = MarketplaceRef::default();

            // When
            let contract = client
                .instantiate("marketplace", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let call_builder = contract.call_builder::<Marketplace>();

            // Then
            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::alice(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let mut constructor = MarketplaceRef::new(false);
            let contract = client
                .instantiate("marketplace", &ink_e2e::bob(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let mut call_builder = contract.call_builder::<Marketplace>();

            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::bob(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = call_builder.flip();
            let _flip_result = client
                .call(&ink_e2e::bob(), &flip)
                .submit()
                .await
                .expect("flip failed");

            // Then
            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::bob(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod marketplace {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct Marketplace {
        usuarios: Mapping<AccountId, Usuario>, // (id_usuario, datos_usuario) este capaz tmbn tenga
        // que ser un vec y un mapping aparte, depende de lo que necesitemos
        // pq si queremos obtener todos los usuarios y mostrarlos sonamos

        // storage general y mapping para mejorar performance
        publicaciones: Vec<Publicacion>,
        ordenes_compra: Vec<OrdenCompra>,
        publicaciones_mapping: Mapping<AccountId, Vec<u32>>, // (id_vendedor, id's publicaciones)
        ordenes_compra_mapping: Mapping<AccountId, Vec<OrdenCompra>>, // (id_comprador, id's ordenes)
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug)]
    pub enum ErrorSistema {
        UsuarioNoRegistrado,
        UsuarioYaRegistrado,
        UsuarioNoEsVendedor,
        UsuarioNoEsComprador,
        VendedorNoExistente,
        VendedorSinPublicaciones,
        PublicacionSinStock,
        PublicacionNoExistente,
        OverflowIdProducto,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub struct Usuario {
        account_id: AccountId,
        username: String,
        rol: Rol,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum Rol {
        Comprador,
        Vendedor,
        Ambos,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub struct Publicacion {
        vendedor_id: AccountId,
        nombre_producto: String,
        descripcion: String,
        precio: u64,
        categoria: Categoria,
        stock: u64,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum Categoria {
        Computacion,
        Ropa,
        Herramientas,
        Muebles,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub struct OrdenCompra {
        estado: Estado,
        publicacion: Publicacion,
        comprador_id: AccountId,
        peticion_cancelacion: bool, // La peticion la hace el comprador, el vendedor acepta, esta
                                    // logica se maneja en el método (?)
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone)]
    pub enum Estado {
        Pendiente,
        Enviada,
        Recibida,
        Cancelada,
    }

    impl Default for Marketplace {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Marketplace {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                usuarios: Default::default(),
                publicaciones: Default::default(),
                ordenes_compra: Default::default(),
                publicaciones_mapping: Default::default(),
                ordenes_compra_mapping: Default::default(),
            }
        }

        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new()
        }

        #[ink(message)]
        pub fn get_usuario(&self) -> Result<Usuario, ErrorSistema> {
            self.usuarios
                .get(self.env().caller())
                .ok_or(ErrorSistema::UsuarioNoRegistrado)
        }

        #[ink(message)]
        pub fn registrar_usuario(
            &mut self,
            username: String,
            rol: Rol,
        ) -> Result<Usuario, ErrorSistema> {
            if self.usuarios.get(self.env().caller()).is_some() {
                return Err(ErrorSistema::UsuarioYaRegistrado);
            };

            let usuario = Usuario {
                account_id: self.env().caller(),
                username,
                rol,
            };

            self.usuarios.insert(self.env().caller(), &usuario);

            Ok(usuario)
        }

        #[ink(message)]
        pub fn publicar(&mut self, publicacion: Publicacion) -> Result<Publicacion, ErrorSistema> {
            let usuario = self.get_usuario()?;
            usuario.es_vendedor()?;

            // agrega al vector total y el id al mapping
            self.publicaciones.push(publicacion.clone());
            let mut publicaciones_vendedor = self
                .publicaciones_mapping
                .get(usuario.account_id)
                .unwrap_or_default();
            publicaciones_vendedor.push(self.publicaciones.len() as u32 - 1); // agrega el index de la publicacion
            self.publicaciones_mapping
                .insert(usuario.account_id, &publicaciones_vendedor);

            Ok(publicacion)
        }

        #[ink(message)]
        pub fn get_publicaciones_vendedor(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            let usuario = self.get_usuario()?;
            usuario.es_vendedor()?;
            let ids_publicaciones_vendedor = self
                .publicaciones_mapping
                .get(usuario.account_id)
                .unwrap_or_default();

            let publicaciones_vendedor = ids_publicaciones_vendedor
                .iter()
                .filter_map(|&i| self.publicaciones.get(i as usize))
                .cloned()
                .collect();

            Ok(publicaciones_vendedor)
        }

        #[ink(message)]
        pub fn ordenar_compra(
            &mut self,
            idx_publicacion: u32,
        ) -> Result<OrdenCompra, ErrorSistema> {
            // validaciones de usuario
            let usuario = self.get_usuario()?;
            usuario.es_comprador()?;

            // buscar publicacion, descrementar stock y aplicarlo
            let publicacion = self
                .publicaciones
                .get(idx_publicacion as usize)
                .cloned()
                .ok_or(ErrorSistema::PublicacionNoExistente)?;
            publicacion
                .stock
                .checked_sub(1)
                .expect("TODO: Aca devolver error custom Publicacion sin stock"); // TODO
            self.publicaciones[idx_publicacion as usize] = publicacion.clone();

            // crear orden de compra
            let orden_compra = OrdenCompra {
                estado: Estado::Pendiente,
                publicacion,
                comprador_id: usuario.account_id,
                peticion_cancelacion: false,
            };

            let mut ordenes_compra_comprador = self
                .ordenes_compra_mapping
                .get(usuario.account_id)
                .unwrap_or_default();
            ordenes_compra_comprador.push(orden_compra.clone());
            self.ordenes_compra_mapping
                .insert(usuario.account_id, &ordenes_compra_comprador);

            Ok(orden_compra)
        }

        #[ink(message)]
        pub fn get_ordenes_comprador(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            let usuario = self.get_usuario()?;
            usuario.es_comprador()?;
            let ordenes_compra = self
                .ordenes_compra
                .get(usuario.account_id)
                .unwrap_or_default();
            Ok(ordenes_compra)
        }
    }

    impl Publicacion {
        pub fn new(
            id: u64,
            vendedor_id: AccountId,
            nombre_producto: String,
            descripcion: String,
            precio: u64,
            categoria: Categoria,
            stock: u64,
        ) -> Publicacion {
            Publicacion {
                id,
                vendedor_id,
                nombre_producto,
                descripcion,
                precio,
                categoria,
                stock,
            }
        }
    }

    impl Usuario {
        fn es_vendedor(&self) -> Result<bool, ErrorSistema> {
            if matches!(self.rol, Rol::Comprador) {
                Err(ErrorSistema::UsuarioNoEsVendedor)
            } else {
                Ok(true)
            }
        }

        fn es_comprador(&self) -> Result<bool, ErrorSistema> {
            if matches!(self.rol, Rol::Vendedor) {
                Err(ErrorSistema::UsuarioNoEsComprador)
            } else {
                Ok(true)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        mod tests_es_vendedor {
            use super::*;

            #[test]
            fn test_es_vendedor_true_vendedor() {
                let usuario = Usuario {
                    account_id: 22,
                    username: agustin22,
                    rol: Rol::Vendedor,
                };

                assert_eq!(usuario.es_vendedor(), true);
            }

            #[test]
            fn test_es_vendedor_true_ambos() {
                let usuario = Usuario {
                    account_id: 22,
                    username: agustin22,
                    rol: Rol::Ambos,
                };

                assert_eq!(usuario.es_vendedor(), true);
            }

            #[test]
            fn test_es_vendedor_false() {
                let usuario = Usuario {
                    account_id: 22,
                    username: agustin22,
                    rol: Rol::Comprador,
                };

                assert_eq!(usuario.es_vendedor(), false);
            }
        }

        mod tests_es_comprador {
            use super::*;

            #[test]
            fn test_es_comprador_true_comprador() {
                let usuario = Usuario {
                    account_id: 22,
                    username: agustin22,
                    rol: Rol::Comprador,
                };

                assert_eq!(usuario.es_comprador(), true);
            }

            #[test]
            fn test_es_comprador_true_ambos() {
                let usuario = Usuario {
                    account_id: 22,
                    username: agustin22,
                    rol: Rol::Ambos,
                };

                assert_eq!(usuario.es_comprador(), true);
            }

            #[test]
            fn test_es_comprador_false() {
                let usuario = Usuario {
                    account_id: 22,
                    username: agustin22,
                    rol: Rol::Vendedor,
                };

                assert_eq!(usuario.es_comprador(), false);
            }
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

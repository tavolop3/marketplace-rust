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
        ordenes_compra_mapping: Mapping<AccountId, Vec<u32>>, // (id_comprador, id's ordenes)
                                                             // u32 parece ser la mejor opción, usize no existe en ink porque depende de la arquitectura
                                                             // u64 incrementaría los costos de transacción
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
        UnderflowPublicaciones,
        UnderflowOrdenes,
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
        id_publicacion: u64,
        nombre_producto: String,
        descripcion: String,
        precio: u64,
        categoria: Categoria,
        stock: u64,
        vendedor_id: AccountId,
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

        //Retorna los datos de un usuario si existe en el sistema
        #[ink(message)]
        pub fn get_usuario(&self) -> Result<Usuario, ErrorSistema> {
            self._get_usuario()
        }

        //Funcion prueba get_usuario()
        fn _get_usuario(&self) -> Result<Usuario, ErrorSistema> {
            self.usuarios
                .get(self.env().caller())
                .ok_or(ErrorSistema::UsuarioNoRegistrado)
        }

        //Registra usuarios que no estan en el sistema
        #[ink(message)]
        pub fn registrar_usuario(&mut self,username: String,rol: Rol) -> Result<Usuario, ErrorSistema> {
            self._registrar_usuario(username,rol)
        }

        //Funcion prueba registrar_usuario()
        fn _registrar_usuario(&mut self,username: String,rol: Rol) -> Result<Usuario, ErrorSistema> {
            //Verifica si el usuario ya esta registrado
            if self.usuarios.get(self.env().caller()).is_some() {
                return Err(ErrorSistema::UsuarioYaRegistrado);
            };

            //Crea el nuevo usuario
            let usuario = Usuario {
                account_id: self.env().caller(),
                username,
                rol,
            };

            //Almacena el nuevo usuario en el sistema
            self.usuarios.insert(self.env().caller(), &usuario);

            Ok(usuario)
        }

        //Crea una publicacion
        #[ink(message)]
        pub fn publicar(&mut self, nombre_producto:String, descripcion:String, precio:u64, categoria:Categoria, stock:u64) -> Result<Publicacion, ErrorSistema> {
            self._publicar(nombre_producto,descripcion,precio,categoria,stock)
        }

        //Funcion prueba publicar()
        fn _publicar(&mut self, nombre_producto:String, descripcion:String, precio:u64, categoria:Categoria, stock:u64) -> Result<Publicacion, ErrorSistema> {
            //Validacion de usuario
            let usuario = self.get_usuario()?;
            usuario.es_vendedor()?;

            //Crea la publicacion
            let publicacion = Publicacion::new(
                self.publicaciones.len() as u64,
                nombre_producto,
                descripcion,
                precio,
                categoria,
                stock,
                usuario.account_id
            );

            //Agrega la publicacion al sistema
            self.publicaciones.push(publicacion.clone());
            //Agrega el index de la publicacion al vector personal del vendedor
            let mut publicaciones_vendedor = self
                .publicaciones_mapping
                .get(usuario.account_id)
                .unwrap_or_default();

            let index_pub = (self.publicaciones.len() as u32).checked_sub(1).ok_or(ErrorSistema::UnderflowPublicaciones)?; // Calcula el index
            publicaciones_vendedor.push(index_pub); // Agrega el index de la publicacion

            //Almacena el vector de indexs del usuario
            self.publicaciones_mapping
                .insert(usuario.account_id, &publicaciones_vendedor);

            Ok(publicacion)
        }

        //Retorna las publicaciones del vendedor solicitante
        #[ink(message)]
        pub fn get_publicaciones_vendedor(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            self._get_publicaciones_vendedor()
        }

        //Funcion prueba get_publicaciones_vendedor()
        fn _get_publicaciones_vendedor(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            //Validacion de usuario
            let usuario = self.get_usuario()?;
            usuario.es_vendedor()?;

            //Obtiene el vector con ids de publicaciones del vendedor
            let ids_publicaciones_vendedor = self
                .publicaciones_mapping
                .get(usuario.account_id)
                .unwrap_or_default();

            //Recorre las publicaciones del sistema y arma un vector con las
            //publicaciones del vendedor solicitante
            let publicaciones_vendedor = ids_publicaciones_vendedor
                .iter()
                .filter_map(|&i| self.publicaciones.get(i as usize))
                .cloned()
                .collect();

            Ok(publicaciones_vendedor)
        }

        //Retorna las publicaciones de todos los vendedores
        #[ink(message)]
        pub fn get_publicaciones(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            self._get_publicaciones()
        }

        //Funcion prueba get_publicaciones()
        fn _get_publicaciones(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            self.get_usuario()?;
            Ok(self.publicaciones.clone())
        }

        //Crea una orden de compra
        #[ink(message)]
        pub fn ordenar_compra(&mut self,idx_publicacion: u32) -> Result<OrdenCompra, ErrorSistema> {
            self._ordenar_compra(idx_publicacion)
        }

        //Funcion prueba ordenar_compra()
        fn _ordenar_compra(&mut self,idx_publicacion: u32) -> Result<OrdenCompra, ErrorSistema> {
            // validaciones de usuario
            let usuario = self.get_usuario()?;
            usuario.es_comprador()?;

            //Buscar publicacion
            let publicacion = self
                .publicaciones
                .get_mut(idx_publicacion as usize)
                .cloned()
                .ok_or(ErrorSistema::PublicacionNoExistente)?;
            //Decrementar Stock
            publicacion
                .stock
                .checked_sub(1)
                .ok_or(ErrorSistema::PublicacionSinStock)?;
            //Actualizar publicacion
            self.publicaciones[idx_publicacion as usize] = publicacion.clone();

            // crear orden de compra
            let orden_compra = OrdenCompra {
                estado: Estado::Pendiente,
                publicacion,
                comprador_id: usuario.account_id,
                peticion_cancelacion: false,
            };

            //Agrega la orden de compra al sistema
            self.ordenes_compra.push(orden_compra.clone());
            //Agrega el index de la orden de compra al vector personal del comprador
            let mut ordenes_compra_comprador = self
                .ordenes_compra_mapping
                .get(usuario.account_id)
                .unwrap_or_default();

            let index_ord = (self.ordenes_compra.len() as u32).checked_sub(1).ok_or(ErrorSistema::UnderflowOrdenes)?; // Calcula el index
            ordenes_compra_comprador.push(index_ord); // Agrega el index de la orden de compra

            //Almacena el vector de indexs del usuario
            self.ordenes_compra_mapping
                .insert(usuario.account_id, &ordenes_compra_comprador);

            Ok(orden_compra)
        }

        //Retorna las ordenes de compra del comprador solicitante
        #[ink(message)]
        pub fn get_ordenes_comprador(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            self._get_ordenes_comprador()
        }

        //Funcion prueba get_ordenes_comprador()
        fn _get_ordenes_comprador(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            //Validacion de usuario
            let usuario = self.get_usuario()?;
            usuario.es_comprador()?;

            //Obtiene el vector con ids de ordenes de compra del comprador
            let ids_ordenes_compra_comprador = self
                .ordenes_compra_mapping
                .get(usuario.account_id)
                .unwrap_or_default();

            //Recorre las ordenes de compra del sistema y arma un vector con las
            //ordenes de compra del comprador solicitante
            let ordenes_compra_comprador = ids_ordenes_compra_comprador
                .iter()
                .filter_map(|&i| self.ordenes_compra.get(i as usize))
                .cloned()
                .collect();

            Ok(ordenes_compra_comprador)
        }

        //Retorna las ordenes de compra de todos los compradores
        #[ink(message)]
        pub fn get_ordenes(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            self._get_ordenes()
        }

        //Funcion prueba get_ordenes
        fn _get_ordenes(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            self.get_usuario()?;
            Ok(self.ordenes_compra.clone())
        }
    }

    impl Publicacion {
        pub fn new(
            id_publicacion: u64,
            nombre_producto: String,
            descripcion: String,
            precio: u64,
            categoria: Categoria,
            stock: u64,
            vendedor_id: AccountId,
        ) -> Publicacion {
            Publicacion {
                id_publicacion,
                nombre_producto,
                descripcion,
                precio,
                categoria,
                stock,
                vendedor_id,
            }
        }
    }

    impl Usuario {
        //Valida que el usuario tenga rol Vendedor o Ambos
        fn es_vendedor(&self) -> Result<bool, ErrorSistema> {
            if matches!(self.rol, Rol::Comprador) {
                Err(ErrorSistema::UsuarioNoEsVendedor)
            } else {
                Ok(true)
            }
        }

        //Valida que el usuario tenga rol Comprador o Ambos
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
            fn tests_es_vendedor_true_vendedor() {
                let usuario = Usuario {
                    account_id: AccountId::from([0xAA; 32]),
                    username: "agustin22".to_string(),
                    rol: Rol::Vendedor,
                };

                assert_eq!(usuario.es_vendedor().is_ok(), true);
            }

            #[test]
            fn tests_es_vendedor_true_ambos() {
                let usuario = Usuario {
                    account_id: AccountId::from([0xAA; 32]),
                    username: "agustin22".to_string(),
                    rol: Rol::Ambos,
                };

                assert_eq!(usuario.es_vendedor().is_ok(), true);
            }

            #[test]
            fn tests_es_vendedor_false() {
                let usuario = Usuario {
                    account_id: AccountId::from([0xAA; 32]),
                    username: "agustin22".to_string(),
                    rol: Rol::Comprador,
                };

                assert_eq!(usuario.es_vendedor().is_ok(), false);
            }
        }

        mod tests_es_comprador {
            use super::*;

            #[test]
            fn tests_es_comprador_true_comprador() {
                let usuario = Usuario {
                    account_id: AccountId::from([0xAA; 32]),
                    username: "agustin22".to_string(),
                    rol: Rol::Comprador,
                };

                assert_eq!(usuario.es_comprador().is_ok(), true);
            }

            #[test]
            fn tests_es_comprador_true_ambos() {
                let usuario = Usuario {
                    account_id: AccountId::from([0xAA; 32]),
                    username: "agustin22".to_string(),
                    rol: Rol::Ambos,
                };

                assert_eq!(usuario.es_comprador().is_ok(), true);
            }

            #[test]
            fn tests_es_comprador_false() {
                let usuario = Usuario {
                    account_id: AccountId::from([0xAA; 32]),
                    username: "agustin22".to_string(),
                    rol: Rol::Vendedor,
                };

                assert_eq!(usuario.es_comprador().is_ok(), false);
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

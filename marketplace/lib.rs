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
    #[derive(Debug, PartialEq)]
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
    #[derive(Debug, Clone, PartialEq)]
    pub struct Usuario {
        username: String,
        rol: Rol,
        account_id: AccountId,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Rol {
        Comprador,
        Vendedor,
        Ambos,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone, PartialEq)]
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
    #[derive(Debug, Clone, PartialEq)]
    pub enum Categoria {
        Computacion,
        Ropa,
        Herramientas,
        Muebles,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone, PartialEq)]
    pub struct OrdenCompra {
        estado: Estado,
        publicacion: Publicacion,
        comprador_id: AccountId,
        peticion_cancelacion: bool, // La peticion la hace el comprador, el vendedor acepta, esta
                                    // logica se maneja en el método (?)
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Estado {
        Pendiente,
        Enviada,
        Recibida,
        Cancelada,
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

        //Registra usuarios que no estan en el sistema
        #[ink(message)]
        #[ignore]
        pub fn registrar_usuario(
            &mut self,
            username: String,
            rol: Rol,
        ) -> Result<Usuario, ErrorSistema> {
            self._registrar_usuario(self.env().caller(), username, rol)
        }

        //Funcion prueba registrar_usuario()
        fn _registrar_usuario(
            &mut self,
            caller: AccountId,
            username: String,
            rol: Rol,
        ) -> Result<Usuario, ErrorSistema> {
            //Verifica si el usuario ya esta registrado
            if self.usuarios.get(caller).is_some() {
                return Err(ErrorSistema::UsuarioYaRegistrado);
            };

            //Crea el nuevo usuario
            let usuario = Usuario {
                account_id: caller,
                username,
                rol,
            };

            //Almacena el nuevo usuario en el sistema
            self.usuarios.insert(caller, &usuario);

            Ok(usuario)
        }

        //Retorna los datos de un usuario si existe en el sistema
        #[ink(message)]
        #[ignore]
        pub fn get_usuario(&self) -> Result<Usuario, ErrorSistema> {
            self._get_usuario(self.env().caller())
        }

        //Funcion prueba get_usuario()
        fn _get_usuario(&self, caller: AccountId) -> Result<Usuario, ErrorSistema> {
            self.usuarios
                .get(caller)
                .ok_or(ErrorSistema::UsuarioNoRegistrado)
        }

        //Crea una publicacion
        #[ink(message)]
        #[ignore]
        pub fn publicar(
            &mut self,
            nombre_producto: String,
            descripcion: String,
            precio: u64,
            categoria: Categoria,
            stock: u64,
        ) -> Result<Publicacion, ErrorSistema> {
            self._publicar(
                self.env().caller(),
                nombre_producto,
                descripcion,
                precio,
                categoria,
                stock,
            )
        }

        //Funcion prueba publicar()
        fn _publicar(
            &mut self,
            caller: AccountId,
            nombre_producto: String,
            descripcion: String,
            precio: u64,
            categoria: Categoria,
            stock: u64,
        ) -> Result<Publicacion, ErrorSistema> {
            //Validacion de usuario
            let usuario = self._get_usuario(caller)?;
            usuario.es_vendedor()?;

            //Crea la publicacion
            let publicacion = Publicacion::new(
                self.publicaciones.len() as u64,
                nombre_producto,
                descripcion,
                precio,
                categoria,
                stock,
                usuario.account_id,
            );

            //Agrega la publicacion al sistema
            self.publicaciones.push(publicacion.clone());
            //Agrega el index de la publicacion al vector personal del vendedor
            let mut publicaciones_vendedor = self
                .publicaciones_mapping
                .get(usuario.account_id)
                .unwrap_or_default();

            let index_pub = (self.publicaciones.len() as u32)
                .checked_sub(1)
                .ok_or(ErrorSistema::UnderflowPublicaciones)?; // Calcula el index
            publicaciones_vendedor.push(index_pub); // Agrega el index de la publicacion

            //Almacena el vector de indexs del usuario
            self.publicaciones_mapping
                .insert(usuario.account_id, &publicaciones_vendedor);

            Ok(publicacion)
        }

        //Retorna las publicaciones del vendedor solicitante
        #[ink(message)]
        #[ignore]
        pub fn get_publicaciones_vendedor(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            self._get_publicaciones_vendedor(self.env().caller())
        }

        //Funcion prueba get_publicaciones_vendedor()
        fn _get_publicaciones_vendedor(
            &self,
            caller: AccountId,
        ) -> Result<Vec<Publicacion>, ErrorSistema> {
            //Validacion de usuario
            let usuario = self._get_usuario(caller)?;
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
        #[ignore]
        pub fn get_publicaciones(&self) -> Result<Vec<Publicacion>, ErrorSistema> {
            self._get_publicaciones(self.env().caller())
        }

        //Funcion prueba get_publicaciones()
        fn _get_publicaciones(&self, caller: AccountId) -> Result<Vec<Publicacion>, ErrorSistema> {
            self._get_usuario(caller)?;
            Ok(self.publicaciones.clone())
        }

        //Crea una orden de compra
        #[ink(message)]
        #[ignore]
        pub fn ordenar_compra(
            &mut self,
            idx_publicacion: u32,
        ) -> Result<OrdenCompra, ErrorSistema> {
            self._ordenar_compra(self.env().caller(), idx_publicacion)
        }

        //Funcion prueba ordenar_compra()
        fn _ordenar_compra(
            &mut self,
            caller: AccountId,
            idx_publicacion: u32,
        ) -> Result<OrdenCompra, ErrorSistema> {
            // validaciones de usuario
            let usuario = self._get_usuario(caller)?;
            usuario.es_comprador()?;

            //Buscar publicacion
            let mut publicacion = self
                .publicaciones
                .get(idx_publicacion as usize)
                .cloned()
                .ok_or(ErrorSistema::PublicacionNoExistente)?;

            //Decrementar Stock
            publicacion.stock = publicacion
                .stock
                .checked_sub(1)
                .ok_or(ErrorSistema::PublicacionSinStock)?;

            // Reemplazar la publicación modificada

            // Reemplazar la publicación modificada
            self.publicaciones[idx_publicacion as usize] = publicacion.clone();

            // crear orden de compra
            let orden_compra = OrdenCompra {
                estado: Estado::Pendiente,
                publicacion: publicacion.clone(),
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

            let index_ord = (self.ordenes_compra.len() as u32)
                .checked_sub(1)
                .ok_or(ErrorSistema::UnderflowOrdenes)?; // Calcula el index
            ordenes_compra_comprador.push(index_ord); // Agrega el index de la orden de compra

            //Almacena el vector de indexs del usuario
            self.ordenes_compra_mapping
                .insert(usuario.account_id, &ordenes_compra_comprador);

            Ok(orden_compra)
        }

        //Retorna las ordenes de compra del comprador solicitante
        #[ink(message)]
        #[ignore]
        pub fn get_ordenes_comprador(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            self._get_ordenes_comprador(self.env().caller())
        }

        //Funcion prueba get_ordenes_comprador()
        fn _get_ordenes_comprador(
            &self,
            caller: AccountId,
        ) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            //Validacion de usuario
            let usuario = self._get_usuario(caller)?;
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
        #[ignore]
        pub fn get_ordenes(&self) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            self._get_ordenes(self.env().caller())
        }

        //Funcion prueba get_ordenes
        fn _get_ordenes(&self, caller: AccountId) -> Result<Vec<OrdenCompra>, ErrorSistema> {
            self._get_usuario(caller)?;
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

        mod tests_registrar_usuario {
            use super::*;

            #[ink::test]
            fn tests_registrar_usuario_no_registrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                assert_eq!(
                    marketplace
                        ._registrar_usuario(caller, username, rol)
                        .is_ok(),
                    true
                );
            }

            #[ink::test]
            fn tests_registrar_usuario_ya_registrado_error() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                assert_eq!(
                    marketplace
                        ._registrar_usuario(caller.clone(), username.clone(), rol.clone())
                        .is_ok(),
                    true
                );

                let result = marketplace._registrar_usuario(caller, username, rol);

                assert_eq!(result, Err(ErrorSistema::UsuarioYaRegistrado));
            }
        }

        mod tests_get_usuario {
            use super::*;

            #[ink::test]
            fn tests_get_usuario_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller, username, rol);

                assert_eq!(marketplace._get_usuario(caller).is_ok(), true);
            }

            #[ink::test]
            fn tests_get_usuario_no_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let result = marketplace._get_usuario(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }
        }

        mod tests_publicar {
            use super::*;

            #[ink::test]
            fn tests_publicar_correcto() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let nombre_producto = "Remera".to_string();
                let descripcion = "algodon".to_string();
                let precio = 12000;
                let categoria = Categoria::Ropa;
                let stock = 20;

                assert_eq!(
                    marketplace
                        ._publicar(
                            caller,
                            nombre_producto,
                            descripcion,
                            precio,
                            categoria,
                            stock
                        )
                        .is_ok(),
                    true
                );
            }

            #[ink::test]
            fn tests_publicar_usuario_no_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let nombre_producto = "Remera".to_string();
                let descripcion = "algodon".to_string();
                let precio = 12000;
                let categoria = Categoria::Ropa;
                let stock = 20;

                let result = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }

            #[ink::test]
            fn tests_publicar_usuario_no_vendedor() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Comprador;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let nombre_producto = "Remera".to_string();
                let descripcion = "algodon".to_string();
                let precio = 12000;
                let categoria = Categoria::Ropa;
                let stock = 20;

                let result = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                assert_eq!(result, Err(ErrorSistema::UsuarioNoEsVendedor));
            }
        }

        mod tests_get_publicaciones_vendedor {
            use super::*;

            #[ink::test]
            fn tests_get_publicaciones_vendedor_correcto() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let mut nombre_producto = "Remera".to_string();
                let mut descripcion = "algodon".to_string();
                let mut precio = 12000;
                let mut categoria = Categoria::Ropa;
                let mut stock = 20;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                nombre_producto = "Pantalon".to_string();
                descripcion = "Jean".to_string();
                precio = 20000;
                categoria = Categoria::Ropa;
                stock = 5;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                assert_eq!(
                    marketplace._get_publicaciones_vendedor(caller).is_ok(),
                    true
                );

                if let Ok(vec_publicaciones) = marketplace._get_publicaciones_vendedor(caller) {
                    assert_eq!(vec_publicaciones.len(), 2);
                }
            }

            #[ink::test]
            fn tests_get_publicaciones_vendedor_usuario_no_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let result = marketplace._get_publicaciones_vendedor(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }

            #[ink::test]
            fn tests_get_publicaciones_vendedor_usuario_no_vendedor() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Comprador;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let result = marketplace._get_publicaciones_vendedor(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoEsVendedor));
            }
        }

        mod tests_get_publicaciones {
            use super::*;

            #[ink::test]
            fn tests_get_publicaciones_correcto() {
                let mut marketplace = Marketplace::new();

                let caller1 = AccountId::from([0xAA; 32]);
                let username1 = "agustin".to_string();
                let rol1 = Rol::Ambos;

                let caller2 = AccountId::from([0xAA; 32]);
                let username2 = "agustin".to_string();
                let rol2 = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller1.clone(), username1, rol1);
                let _ = marketplace._registrar_usuario(caller2.clone(), username2, rol2);

                let mut nombre_producto = "Remera".to_string();
                let mut descripcion = "algodon".to_string();
                let mut precio = 12000;
                let mut categoria = Categoria::Ropa;
                let mut stock = 20;

                let _ = marketplace._publicar(
                    caller1,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                nombre_producto = "Pantalon".to_string();
                descripcion = "Jean".to_string();
                precio = 20000;
                categoria = Categoria::Ropa;
                stock = 5;

                let _ = marketplace._publicar(
                    caller1,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                nombre_producto = "Notebook".to_string();
                descripcion = "Ryzen 7".to_string();
                precio = 200000;
                categoria = Categoria::Computacion;
                stock = 10;

                let _ = marketplace._publicar(
                    caller2,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                assert_eq!(marketplace._get_publicaciones(caller1).is_ok(), true);

                if let Ok(vec_publicaciones) = marketplace._get_publicaciones(caller1) {
                    assert_eq!(vec_publicaciones.len(), 3);
                }
            }

            #[ink::test]
            fn tests_get_publicaciones_usuario_no_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let result = marketplace._get_publicaciones(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }
        }

        mod tests_ordenar_compra {
            use super::*;

            #[ink::test]
            fn tests_ordenar_compra_correcto() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller, username, rol);

                let nombre_producto = "Remera".to_string();
                let descripcion = "algodon".to_string();
                let precio = 12000;
                let categoria = Categoria::Ropa;
                let stock = 20;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let orden = marketplace._ordenar_compra(caller, 0_u32);
                assert!(orden.is_ok());
                assert!(marketplace.publicaciones[0].stock == 19);
            }

            #[ink::test]
            fn tests_ordenar_compra_usuario_no_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let result = marketplace._ordenar_compra(caller, 0_u32);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }

            #[ink::test]
            fn tests_ordenar_compra_usuario_no_comprador() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Vendedor;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let result = marketplace._ordenar_compra(caller, 0 as u32);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoEsComprador));
            }

            #[ink::test]
            fn tests_ordenar_compra_publicacion_no_existente() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let mut nombre_producto = "Remera".to_string();
                let mut descripcion = "algodon".to_string();
                let mut precio = 12000;
                let mut categoria = Categoria::Ropa;
                let mut stock = 20;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let result = marketplace._ordenar_compra(caller, 1 as u32);

                assert_eq!(result, Err(ErrorSistema::PublicacionNoExistente));
            }

            #[ink::test]
            fn tests_ordenar_compra_publicacion_sin_stock() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller, username, rol);

                let nombre_producto = "Remera".to_string();
                let descripcion = "algodon".to_string();
                let precio = 12000;
                let categoria = Categoria::Ropa;
                let stock = 0;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let result = marketplace._ordenar_compra(caller, 0_u32);

                assert_eq!(result, Err(ErrorSistema::PublicacionSinStock));
            }
        }

        mod tests_get_ordenes_comprador {
            use super::*;

            #[ink::test]
            fn tests_get_ordenes_comprador_correcto() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller, username, rol);

                let mut nombre_producto = "Remera".to_string();
                let mut descripcion = "algodon".to_string();
                let mut precio = 12000;
                let mut categoria = Categoria::Ropa;
                let mut stock = 20;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let _ = marketplace._ordenar_compra(caller, 0_u32);

                nombre_producto = "Pantalon".to_string();
                descripcion = "Jean".to_string();
                precio = 20000;
                categoria = Categoria::Ropa;
                stock = 5;

                let _ = marketplace._publicar(
                    caller,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let _ = marketplace._ordenar_compra(caller, 1_u32);

                assert!(marketplace._get_ordenes_comprador(caller).is_ok());

                if let Ok(vec_ordenes) = marketplace._get_ordenes_comprador(caller) {
                    assert_eq!(vec_ordenes.len(), 2);
                }
            }

            #[ink::test]
            fn tests_get_ordenes_comprador_usuario_no_encontrado() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let result = marketplace._get_ordenes_comprador(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }

            #[ink::test]
            fn tests_get_ordenes_comprador_usuario_no_comprador() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Vendedor;

                let _ = marketplace._registrar_usuario(caller.clone(), username, rol);

                let result = marketplace._get_ordenes_comprador(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoEsComprador));
            }
        }

        mod tests_get_ordenes {
            use super::*;

            #[ink::test]
            fn tests_get_ordenes_correcto() {
                let mut marketplace = Marketplace::new();

                let caller1 = AccountId::from([0xAA; 32]);
                let username1 = "agustin".to_string();
                let rol1 = Rol::Ambos;

                let caller2 = AccountId::from([0xBB; 32]);
                let username2 = "juan".to_string();
                let rol2 = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller1, username1, rol1);
                let _ = marketplace._registrar_usuario(caller2, username2, rol2);

                let mut nombre_producto = "Remera".to_string();
                let mut descripcion = "algodon".to_string();
                let mut precio = 12000;
                let mut categoria = Categoria::Ropa;
                let mut stock = 20;

                let _ = marketplace._publicar(
                    caller1,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let _ = marketplace._ordenar_compra(caller2, 0_u32);

                nombre_producto = "Pantalon".to_string();
                descripcion = "Jean".to_string();
                precio = 20000;
                categoria = Categoria::Ropa;
                stock = 5;

                let _ = marketplace._publicar(
                    caller1,
                    nombre_producto,
                    descripcion,
                    precio,
                    categoria,
                    stock,
                );

                let _ = marketplace._ordenar_compra(caller2, 1_u32);

                assert!(marketplace._get_ordenes(caller1).is_ok());

                if let Ok(vec_ordenes) = marketplace._get_ordenes(caller1) {
                    assert_eq!(vec_ordenes.len(), 2);
                }
            }

            #[ink::test]
            fn tests_get_ordenes_usuario_no_encontrado() {
                let marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);

                let result = marketplace._get_ordenes(caller);

                assert_eq!(result, Err(ErrorSistema::UsuarioNoRegistrado));
            }

            #[ink::test]
            fn tests_get_ordenes_sin_ordenes() {
                let mut marketplace = Marketplace::new();

                let caller = AccountId::from([0xAA; 32]);
                let username = "agustin".to_string();
                let rol = Rol::Ambos;

                let _ = marketplace._registrar_usuario(caller, username, rol);

                let result = marketplace._get_ordenes(caller);

                assert!(result.is_ok());
                if let Ok(vec_ordenes) = result {
                    assert_eq!(vec_ordenes.len(), 0);
                }
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
